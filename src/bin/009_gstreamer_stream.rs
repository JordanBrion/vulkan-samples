extern crate anyhow;
extern crate glib;
extern crate glib_sys;
extern crate gst;
extern crate gstreamer_rtsp_server;
extern crate termion;

use gst::{prelude::*, ClockTime};
use gstreamer_rtsp_server::prelude::{
    RTSPMediaExt, RTSPMediaFactoryExt, RTSPMountPointsExt, RTSPServerExt, RTSPServerExtManual,
};

const WIDTH: usize = 384;
const HEIGHT: usize = 288;
const BYTES_PER_PIXEL: usize = 2;
const FRAME_SIZE: usize = WIDTH * HEIGHT * BYTES_PER_PIXEL; // RGB16

fn create_blue_frame_buffer() -> gst::Buffer {
    let buffer = gst::Buffer::with_size(FRAME_SIZE).unwrap();
    let mut mapinfo = buffer.into_mapped_buffer_writable().unwrap();
    unsafe {
        let ptr = mapinfo.as_mut_ptr() as *mut u16;
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                let current_pixel_ptr = ptr.offset((i * WIDTH + j) as isize);
                *current_pixel_ptr = 0b1111100000011111;
            }
        }
    }
    mapinfo.into_buffer()
}

fn main() {
    gst::init().unwrap();

    let main_loop = glib::MainLoop::new(None, false);

    let server = gstreamer_rtsp_server::RTSPServer::new();
    let mounts = server.mount_points().unwrap();
    let factory = gstreamer_rtsp_server::RTSPMediaFactory::new();
    factory
        .set_launch("( appsrc name=mysrc ! videoconvert ! x264enc ! rtph264pay name=pay0 pt=96 )");

    factory.connect("media-configure", false, |args| {
        println!("media-configure");
        let media = args[1].get::<gstreamer_rtsp_server::RTSPMedia>().unwrap();
        let element = media.element();
        let bin = element.dynamic_cast::<gst::Bin>().unwrap();

        let appsrc = bin
            .by_name_recurse_up("mysrc")
            .unwrap()
            .dynamic_cast::<gstreamer_app::AppSrc>()
            .unwrap();
        appsrc.set_property_from_str("format", "time");
        let appsrc_caps = gst::caps::Caps::builder("video/x-raw")
            .field("format", "RBG16")
            .field("width", 384)
            .field("height", 288)
            // .field("framerate", "60")
            .build();
        appsrc.set_caps(Some(&appsrc_caps));
        // let mut timestamp: ClockTime = Default::default();

        appsrc.connect("need-data", false, |args| {
            println!("pushing data");
            let element: gst::Element = args[0].get::<gst::Element>().unwrap();
            let appsrc = element.dynamic_cast::<gstreamer_app::AppSrc>().unwrap();

            let mut frame_buffer = create_blue_frame_buffer();
            // frame_buffer.set_pts(pts);
            // frame_buffer.set_duration(gst::ClockTime::SECOND);

            // timestamp = timestamp + frame_buffer.duration().unwrap();
            //timestamp + gst::ClockTime::SECOND;

            let ret = appsrc
                .emit_by_name_with_values("push-buffer", &[frame_buffer.into()])
                .unwrap();
            None
        });
        None
    });

    // TODO
    //   gst_rtsp_mount_points_add_factory(mounts, "/test", factory);
    mounts.add_factory("/test", factory);

    let timestamp = gst::ClockTime::default();
    let result = server.attach(None).unwrap();
    main_loop.run();
}
