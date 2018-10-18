// This code opens a camera device on a windows machine, displays to gtk widget using gstreamer
extern crate gtk;
use gtk::prelude::*;
use gtk::Builder;

extern crate glib;

extern crate gio;
use gio::prelude::*;

extern crate gstreamer as gst;
use gst::prelude::*;

use gtk::{Window};

use std::cell::RefCell;
use std::env;
// Stream url: gst-launch-1.0 uridecodebin uri=rtsp://184.72.239.149/vod/mp4:BigBuckBunny_175k.mov ! autovideosink
// To record and display simultaneously
// gst-launch-1.0 videotestsrc ! queue ! x264enc ! h264parse ! tee name=t ! queue ! mp4mux ! filesink location=video.mp4 t. ! queue ! h264parse ! decodebin ! xvimagesink sync=false
fn create_ui(app: &gtk::Application) {
    let pipeline = gst::Pipeline::new(None);
    // let src2 = gst::ElementFactory::make("rtspsrc", None).unwrap();
    let src = gst::ElementFactory::make("ksvideosrc", None).unwrap();
    let vid_convert = gst::ElementFactory::make("videoconvert", None).unwrap();
    let sink = gst::ElementFactory::make("gtksink", None).unwrap();
    let widget = sink.get_property("widget").unwrap().get::<gtk::Widget>().unwrap();


    pipeline.add_many(&[&src, &vid_convert, &sink]).unwrap();
    gst::Element::link_many(&[&src, &vid_convert, &sink]).unwrap();
    // sink.set_property("location", &"out.mkv".to_value())?;

    // glade starts 
    let glade_src = include_str!("gtkbuilder.glade");
    let builder = Builder::new();
    builder.add_from_string(glade_src).expect("Could not load file");
    let window: Window = builder.get_object("window").unwrap();
    let vid_widget_box: gtk::Box = builder.get_object("box_layout").unwrap();
    let label: gtk::Label = builder.get_object("label").unwrap();
    vid_widget_box.pack_start(&widget, true, true, 0);
    // glade ends

    window.show_all();

    app.add_window(&window);

    let pipeline_weak = pipeline.downgrade();
    let timeout_id = gtk::timeout_add(500, move || {
        let pipeline = match pipeline_weak.upgrade() {
            Some(pipeline) => pipeline,
            None => return glib::Continue(true),
        };

        let position = pipeline
            .query_position::<gst::ClockTime>()
            .unwrap_or_else(|| 0.into());
        label.set_text(&format!("Position: {:.0}", position));

        glib::Continue(true)
    });

    let app_weak = app.downgrade();
    window.connect_delete_event(move |_, _| {
        let app = match app_weak.upgrade() {
            Some(app) => app,
            None => return Inhibit(false),
        };

        app.quit();
        Inhibit(false)
    });

    let bus = pipeline.get_bus().unwrap();

    let ret = pipeline.set_state(gst::State::Playing);
    assert_ne!(ret, gst::StateChangeReturn::Failure);

    let app_weak = glib::SendWeakRef::from(app.downgrade());
    bus.add_watch(move |_, msg| {
        use gst::MessageView;

        let app = match app_weak.upgrade() {
            Some(app) => app,
            None => return glib::Continue(false),
        };

        match msg.view() {
            MessageView::Eos(..) => gtk::main_quit(),
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.get_src().map(|s| s.get_path_string()),
                    err.get_error(),
                    err.get_debug()
                );
                app.quit();
            }
            _ => (),
        };

        glib::Continue(true)
    });

    // Pipeline reference is owned by the closure below, so will be
    // destroyed once the app is destroyed
    let timeout_id = RefCell::new(Some(timeout_id));
    app.connect_shutdown(move |_| {
        let ret = pipeline.set_state(gst::State::Null);
        assert_ne!(ret, gst::StateChangeReturn::Failure);

        bus.remove_watch();
        if let Some(timeout_id) = timeout_id.borrow_mut().take() {
            glib::source_remove(timeout_id);
        }
    });
}

fn main() {
    gst::init().unwrap();
    gtk::init().unwrap();

   let app = gtk::Application::new("com.github.basic",
                                            gio::ApplicationFlags::empty())
                                         .expect("Initialization failed...");

    app.connect_activate(create_ui);
    let args = env::args().collect::<Vec<_>>();
    app.run(&args);
}
