#![warn(clippy::all, rust_2018_idioms)]

pub mod app;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::activity::WindowManagerFlags;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    // Disable LAYOUT_IN_SCREEN to keep app from drawing under the status bar
    // winit does not currently do anything with MainEvent::InsetsChanged events
    app.set_window_flags(
        WindowManagerFlags::empty(),
        WindowManagerFlags::LAYOUT_IN_SCREEN,
    );
    // Alternatively we can hide the system bars by setting the app to fullscreen
    //app.set_window_flags(WindowManagerFlags::FULLSCREEN, WindowManagerFlags::empty());

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );
    let mut options = eframe::NativeOptions::default();
    options.event_loop_builder = Some(Box::new(move |builder| {
        builder.with_android_app(app);
    }));

    let res = eframe::run_native(
        "eframe template",
        options,
        Box::new(|cc| Box::new(app::TemplateApp::new(cc))),
    );
    if let Err(e) = res {
        log::error!("{e:?}");
    }
}
