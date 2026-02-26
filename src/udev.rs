use std::time::Duration;

use smithay::{
    backend::{
        drm::DrmNode,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        session::{libseat::LibSeatSession, Session},
        udev::{primary_gpu, UdevBackend, UdevEvent},
    },
    reexports::{calloop::EventLoop, input::Libinput, wayland_server::Display},
};

use crate::state::Clux;

pub fn run_udev() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop: EventLoop<Clux> = EventLoop::try_new()?;
    let display: Display<Clux> = Display::new()?;
    let mut state = Clux::new(&mut event_loop, display);

    let (session, notifier) =
        LibSeatSession::new().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let mut libinput_context = Libinput::new_with_udev(
        LibinputSessionInterface::<LibSeatSession>::from(session.clone()),
    );

    libinput_context
        .udev_assign_seat(&session.seat())
        .map_err(|_| "Failed to assign libinput seat")?;

    let libinput_backend = LibinputInputBackend::new(libinput_context);

    let udev_backend =
        UdevBackend::new(&session.seat()).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    event_loop
        .handle()
        .insert_source(libinput_backend, move |event, _, state| {
            state.process_input_event(event);
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let mut primary_gpu_id = primary_gpu(&session.seat()).unwrap_or_default();

    event_loop
        .handle()
        .insert_source(udev_backend, move |event, _, _state| match event {
            UdevEvent::Added { device_id, path } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    tracing::info!("Udev: DRM node added: {:?} at {:?}", node, path);
                    if primary_gpu_id == Some(path.clone()) || primary_gpu_id.is_none() {
                        primary_gpu_id = Some(path.clone());
                        tracing::info!("Primary GPU selected: {:?}", path);
                    }
                }
            }
            UdevEvent::Changed { device_id } => {
                tracing::info!("Udev: device changed: {:?}", device_id);
            }
            UdevEvent::Removed { device_id } => {
                tracing::info!("Udev: device removed: {:?}", device_id);
            }
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    event_loop
        .handle()
        .insert_source(notifier, move |_, _, _| {})
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    let mut running = true;

    while running {
        let result = event_loop.dispatch(Some(Duration::from_millis(16)), &mut state);
        if result.is_err() {
            running = false;
        } else {
            state.space.refresh();
            state.popups.cleanup();
            let _ = state.display_handle.flush_clients();
        }
    }

    Ok(())
}
