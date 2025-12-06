extern crate anyhow;
extern crate gstreamer;
extern crate gstreamer_rtsp_server;
extern crate gstreamer_video;

use std::fmt::Error;

use gstreamer_rtsp_server::prelude::*;

fn main_loop() -> Result<(), Error> {
    let main_loop = glib::MainLoop::new(None, false);
    let server = server::Server::default();

    let mounts = gstreamer_rtsp_server::RTSPMountPoints::default();
    server.set_mount_points(Some(&mounts));

    // Much like HTTP servers, RTSP servers have multiple endpoints that
    // provide different streams. Here, we ask our server to give
    // us a reference to his list of endpoints, so we can add our
    // test endpoint, providing the pipeline from the cli.
    let mounts = server.mount_points().unwrap();

    // Next, we create our custom factory for the endpoint we want to create.
    // The job of the factory is to create a new pipeline for each client that
    // connects, or (if configured to do so) to reuse an existing pipeline.
    let factory = media_factory::Factory::default();

    // This setting specifies whether each connecting client gets the output
    // of a new instance of the pipeline, or whether all connected clients share
    // the output of the same pipeline.
    // If you want to stream a fixed video you have stored on the server to any
    // client, you would not set this to shared here (since every client wants
    // to start at the beginning of the video). But if you want to distribute
    // a live source, you will probably want to set this to shared, to save
    // computing and memory capacity on the server.
    factory.set_shared(true);

    // Now we add a new mount-point and tell the RTSP server to serve the content
    // provided by the factory we configured above, when a client connects to
    // this specific path.
    mounts.add_factory("/test", factory);

    // Attach the server to our main context.
    // A main context is the thing where other stuff is registering itself for its
    // events (e.g. sockets, GStreamer bus, ...) and the main loop is something that
    // polls the main context for its events and dispatches them to whoever is
    // interested in them. In this example, we only do have one, so we can
    // leave the context parameter empty, it will automatically select
    // the default one.
    let id = server.attach(None).unwrap();

    println!(
        "Stream ready at rtsp://127.0.0.1:{}/test",
        server.bound_port()
    );

    // Start the mainloop. From this point on, the server will start to serve
    // our quality content to connecting clients.
    main_loop.run();

    id.remove();

    Ok(())
}

const WIDTH: usize = 384;
const HEIGHT: usize = 288;
const BYTES_PER_PIXEL: usize = 2;
const FRAME_SIZE: usize = WIDTH * HEIGHT * BYTES_PER_PIXEL; // RGB16

fn create_blue_frame_buffer() -> gstreamer::Buffer {
    let buffer = gstreamer::Buffer::with_size(FRAME_SIZE).unwrap();
    let mut mapinfo = buffer.into_mapped_buffer_writable().unwrap();
    unsafe {
        let ptr = mapinfo.as_mut_ptr() as *mut u16;
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                let current_pixel_ptr = ptr.offset((i * WIDTH + j) as isize);
                *current_pixel_ptr = !0b1111100000011111;
            }
        }
    }
    mapinfo.into_buffer()
}

// Our custom media factory that creates a media input manually
mod media_factory {
    use gstreamer_rtsp_server::subclass::prelude::*;

    use super::*;

    // In the imp submodule we include the actual implementation
    mod imp {
        use gstreamer_video::VideoFrameExt;

        use super::*;

        // This is the private data of our factory
        #[derive(Default)]
        pub struct Factory {}

        // This trait registers our type with the GObject object system and
        // provides the entry points for creating a new instance and setting
        // up the class data
        #[glib::object_subclass]
        impl ObjectSubclass for Factory {
            const NAME: &'static str = "RsRTSPMediaFactory";
            type Type = super::Factory;
            type ParentType = gstreamer_rtsp_server::RTSPMediaFactory;
        }

        // Implementation of glib::Object virtual methods
        impl ObjectImpl for Factory {
            fn constructed(&self) {
                self.parent_constructed();

                let factory = self.obj();
                // All media created by this factory are our custom media type. This would
                // not require a media factory subclass and can also be called on the normal
                // RTSPMediaFactory.
                factory.set_media_gtype(super::media::Media::static_type());
            }
        }

        // Implementation of gstreamer_rtsp_server::RTSPMediaFactory virtual methods
        impl RTSPMediaFactoryImpl for Factory {
            fn create_element(
                &self,
                _url: &gstreamer_rtsp_server::gst_rtsp::RTSPUrl,
            ) -> Option<gstreamer::Element> {
                // Create a simple VP8 videotestsrc input
                let bin = gstreamer::Bin::default();

                let video_info = gstreamer_video::VideoInfo::builder(
                    gstreamer_video::VideoFormat::Rgb16,
                    384 as u32,
                    288 as u32,
                )
                .fps(gstreamer::Fraction::new(2, 1))
                .build()
                .expect("Failed to create video info");

                let appsrc = gstreamer_app::AppSrc::builder()
                    .name("mysrc")
                    .caps(&video_info.to_caps().unwrap())
                    .format(gstreamer::Format::Time)
                    .build();

                appsrc.set_callbacks(
                    // Since our appsrc element operates in pull mode (it asks us to provide data),
                    // we add a handler for the need-data callback and provide new data from there.
                    // In our case, we told gstreamer that we do 2 frames per second. While the
                    // buffers of all elements of the pipeline are still empty, this will be called
                    // a couple of times until all of them are filled. After this initial period,
                    // this handler will be called (on average) twice per second.
                    gstreamer_app::AppSrcCallbacks::builder()
                        .need_data(move |appsrc, _| {
                            // appsrc already handles the error here
                            let _ = appsrc.push_buffer(create_blue_frame_buffer());
                        })
                        .build(),
                );

                let video_convert = gstreamer::ElementFactory::make("videoconvert")
                    .name("video_convert")
                    .build()
                    .unwrap();
                let x264enc = gstreamer::ElementFactory::make("x264enc")
                    .name("x264enc")
                    .build()
                    .unwrap();
                let rtph264pay = gstreamer::ElementFactory::make("rtph264pay")
                    .name("pay0")
                    .property_from_str("pt", "96")
                    .build()
                    .unwrap();
                bin.add_many([appsrc.upcast_ref(), &video_convert, &x264enc, &rtph264pay])
                    .unwrap();
                gstreamer::Element::link_many([
                    appsrc.upcast_ref(),
                    &video_convert,
                    &x264enc,
                    &rtph264pay,
                ])
                .unwrap();
                Some(bin.upcast())
            }
        }
    }

    // This here defines the public interface of our factory and implements
    // the corresponding traits so that it behaves like any other RTSPMediaFactory
    glib::wrapper! {
        pub struct Factory(ObjectSubclass<imp::Factory>) @extends gstreamer_rtsp_server::RTSPMediaFactory;
    }

    impl Default for Factory {
        // Creates a new instance of our factory
        fn default() -> Factory {
            glib::Object::new()
        }
    }
}

// Our custom media subclass that adds a custom attribute to the SDP returned by DESCRIBE
mod media {
    use gstreamer_rtsp_server::subclass::prelude::*;

    // In the imp submodule we include the actual implementation
    mod imp {
        use super::*;

        // This is the private data of our media
        #[derive(Default)]
        pub struct Media {}

        // This trait registers our type with the GObject object system and
        // provides the entry points for creating a new instance and setting
        // up the class data
        #[glib::object_subclass]
        impl ObjectSubclass for Media {
            const NAME: &'static str = "RsRTSPMedia";
            type Type = super::Media;
            type ParentType = gstreamer_rtsp_server::RTSPMedia;
        }

        // Implementation of glib::Object virtual methods
        impl ObjectImpl for Media {}

        // Implementation of gstreamer_rtsp_server::RTSPMedia virtual methods
        impl RTSPMediaImpl for Media {
            fn setup_sdp(
                &self,
                sdp: &mut gstreamer_rtsp_server::gst_sdp::SDPMessageRef,
                info: &gstreamer_rtsp_server::subclass::SDPInfo,
            ) -> Result<(), gstreamer::LoggableError> {
                self.parent_setup_sdp(sdp, info)?;

                sdp.add_attribute("my-custom-attribute", Some("has-a-value"));

                Ok(())
            }
        }
    }

    // This here defines the public interface of our factory and implements
    // the corresponding traits so that it behaves like any other RTSPMedia
    glib::wrapper! {
        pub struct Media(ObjectSubclass<imp::Media>) @extends gstreamer_rtsp_server::RTSPMedia;
    }
}

// Our custom RTSP server subclass that reports when clients are connecting and uses
// our custom RTSP client subclass for each client
mod server {
    use gstreamer_rtsp_server::subclass::prelude::*;

    use super::*;

    // In the imp submodule we include the actual implementation
    mod imp {
        use super::*;

        // This is the private data of our server
        #[derive(Default)]
        pub struct Server {}

        // This trait registers our type with the GObject object system and
        // provides the entry points for creating a new instance and setting
        // up the class data
        #[glib::object_subclass]
        impl ObjectSubclass for Server {
            const NAME: &'static str = "RsRTSPServer";
            type Type = super::Server;
            type ParentType = gstreamer_rtsp_server::RTSPServer;
        }

        // Implementation of glib::Object virtual methods
        impl ObjectImpl for Server {}

        // Implementation of gstreamer_rtsp_server::RTSPServer virtual methods
        impl RTSPServerImpl for Server {
            fn create_client(&self) -> Option<gstreamer_rtsp_server::RTSPClient> {
                let server = self.obj();
                let client = super::client::Client::default();

                // Duplicated from the default implementation
                client.set_session_pool(server.session_pool().as_ref());
                client.set_mount_points(server.mount_points().as_ref());
                client.set_auth(server.auth().as_ref());
                client.set_thread_pool(server.thread_pool().as_ref());

                Some(client.upcast())
            }

            fn client_connected(&self, client: &gstreamer_rtsp_server::RTSPClient) {
                self.parent_client_connected(client);
                println!("Client {client:?} connected");
            }
        }
    }

    // This here defines the public interface of our factory and implements
    // the corresponding traits so that it behaves like any other RTSPServer
    glib::wrapper! {
        pub struct Server(ObjectSubclass<imp::Server>) @extends gstreamer_rtsp_server::RTSPServer;
    }

    impl Default for Server {
        // Creates a new instance of our factory
        fn default() -> Server {
            glib::Object::new()
        }
    }
}

// Our custom RTSP client subclass.
mod client {
    use gstreamer_rtsp_server::subclass::prelude::*;

    // In the imp submodule we include the actual implementation
    mod imp {
        use super::*;

        // This is the private data of our server
        #[derive(Default)]
        pub struct Client {}

        // This trait registers our type with the GObject object system and
        // provides the entry points for creating a new instance and setting
        // up the class data
        #[glib::object_subclass]
        impl ObjectSubclass for Client {
            const NAME: &'static str = "RsRTSPClient";
            type Type = super::Client;
            type ParentType = gstreamer_rtsp_server::RTSPClient;
        }

        // Implementation of glib::Object virtual methods
        impl ObjectImpl for Client {}

        // Implementation of gstreamer_rtsp_server::RTSPClient virtual methods
        impl RTSPClientImpl for Client {
            fn closed(&self) {
                let client = self.obj();
                self.parent_closed();
                println!("Client {client:?} closed");
            }

            fn describe_request(&self, ctx: &gstreamer_rtsp_server::RTSPContext) {
                self.parent_describe_request(ctx);
                let request_uri = ctx.uri().unwrap().request_uri();
                println!("Describe request for uri: {request_uri:?}");
            }
        }
    }

    // This here defines the public interface of our factory and implements
    // the corresponding traits so that it behaves like any other RTSPClient
    glib::wrapper! {
        pub struct Client(ObjectSubclass<imp::Client>) @extends gstreamer_rtsp_server::RTSPClient;
    }

    impl Default for Client {
        // Creates a new instance of our factory
        fn default() -> Client {
            glib::Object::new()
        }
    }
}

fn main() {
    gstreamer::init().unwrap();
    main_loop().unwrap();
}
