extern crate glib;
extern crate gstreamer as gst;
extern crate gstreamer_rtsp as gst_rtsp;
extern crate gstreamer_rtsp_server as gst_rtsp_server;

use glib::prelude::*;
use gst_rtsp_server::prelude::*;
use gst_rtsp_server::subclass::prelude::*;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;

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

// 1. Implementation Module
mod imp {
    use super::*;

    #[derive(Default)]
    pub struct CustomFactory {
        pub value666: Mutex<i8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CustomFactory {
        const NAME: &'static str = "CustomRTSPMediaFactory";
        type Type = super::CustomFactory; // Refers to the wrapper below
        type ParentType = gst_rtsp_server::RTSPMediaFactory;
    }

    impl ObjectImpl for CustomFactory {}

    impl RTSPMediaFactoryImpl for CustomFactory {
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

// 2. Public Wrapper for the GObject
glib::wrapper! {
    pub struct CustomFactory(ObjectSubclass<imp::CustomFactory>)
        @extends gst_rtsp_server::RTSPMediaFactory;
}

impl CustomFactory {
    pub fn new(val: i8) -> Self {
        let mut obj: Self = glib::Object::new();
        // Set the initial value
        obj.set_value666(val);
        obj
    }

    // 3. Add helper methods to the public wrapper to get/set the value
    pub fn set_value666(&mut self, val: i8) {
        let mut value = self.imp().value666.lock().unwrap();
        *value = val;
    }

    pub fn value666(&self) -> i8 {
        *self.imp().value666.lock().unwrap()
    }
}

fn main() {
    // 3. Initialize GStreamer
    gst::init().expect("Failed to initialize GStreamer");

    let main_loop = glib::MainLoop::new(None, false);
    let server = gst_rtsp_server::RTSPServer::new();
    let mounts = server.mount_points().expect("Could not get mount points");

    // 4. Set up the custom factory
    let factory = CustomFactory::new(56);
    factory.set_shared(true); // Share the same pipeline across clients

    println!("value jordan {}", factory.value666());

    // 5. Attach the factory to a path
    mounts.add_factory("/test", factory);

    // 6. Start the server on default port 8554
    let _id = server.attach(None).expect("Failed to attach server");

    println!("Server running! Use: ffplay rtsp://127.0.0.1:8554/test");
    main_loop.run();
}
