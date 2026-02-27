use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Format, Fourcc, Modifier,
        },
        drm::{
            compositor::{DrmCompositor, FrameFlags},
            exporter::gbm::{GbmFramebufferExporter, NodeFilter},
            DrmDevice, DrmDeviceFd, DrmEvent,
        },
        egl::{EGLContext, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{gles::GlesRenderer, Color32F, ImportMem},
        session::{libseat::LibSeatSession, Session},
        udev::{primary_gpu, UdevBackend},
    },
    desktop::{space::space_render_elements, Window},
    output::{Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::EventLoop,
        drm::control::{connector::State as ConnectorState, Device},
        input::Libinput,
        wayland_server::Display,
    },
    utils::{DeviceFd, Point, Size},
};

use crate::state::Clux;

pub fn run_udev() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop: EventLoop<Clux> = EventLoop::try_new()?;
    let display: Display<Clux> = Display::new()?;
    let mut state = Clux::new(&mut event_loop, display);

    let (mut session, notifier) = LibSeatSession::new()?;

    let mut libinput_context = Libinput::new_with_udev(
        LibinputSessionInterface::<LibSeatSession>::from(session.clone()),
    );
    libinput_context
        .udev_assign_seat(&session.seat())
        .map_err(|_| "Failed to assign libinput seat")?;
    let input_backend = LibinputInputBackend::new(libinput_context);

    event_loop
        .handle()
        .insert_source(input_backend, move |event, _, state| {
            state.process_input_event(event);
        })?;

    let primary_gpu_path = primary_gpu(&session.seat())?.unwrap_or_else(|| "/dev/dri/card0".into());

    let fd = session.open(
        &primary_gpu_path,
        smithay::reexports::rustix::fs::OFlags::RDWR,
    )?;
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));

    let (mut drm, drm_notifier) = DrmDevice::new(drm_fd.clone(), false)?;
    let gbm = GbmDevice::new(drm_fd.clone())?;
    let egl_display = unsafe { EGLDisplay::new(gbm.clone()) }?;
    let egl_context = EGLContext::new(&egl_display)?;
    let renderer = unsafe { GlesRenderer::new(egl_context)? };
    let renderer = Arc::new(Mutex::new(renderer));

    let allocator = GbmAllocator::new(
        gbm.clone(),
        GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
    );

    let mut compositors = HashMap::new();

    let res_handles = drm.resource_handles()?;
    for conn in res_handles.connectors() {
        let info = drm.get_connector(*conn, true)?;
        if info.state() == ConnectorState::Connected {
            let name = format!("{}-{}", info.interface().as_str(), info.interface_id());
            let output = Output::new(
                name,
                PhysicalProperties {
                    size: info
                        .size()
                        .map(|(w, h)| (w as i32, h as i32).into())
                        .unwrap_or_default(),
                    subpixel: Subpixel::Unknown,
                    make: "Unknown".into(),
                    model: "Generic".into(),
                    serial_number: "Unknown".into(),
                },
            );

            let mode = info.modes()[0];
            output.set_preferred(mode.into());
            output.change_current_state(Some(mode.into()), None, None, Some(Point::from((0, 0))));

            state.map_output(&output, (0, 0).into());

            let encoder_handle = info.current_encoder().ok_or("No encoder")?;
            let encoder = drm.get_encoder(encoder_handle)?;
            let crtc = encoder.crtc().ok_or("No CRTC")?;
            let surface = drm.create_surface(crtc, mode, &[*conn])?;

            let exporter = GbmFramebufferExporter::new(gbm.clone(), NodeFilter::None);

            let compositor = DrmCompositor::new(
                &output,
                surface,
                None,
                allocator.clone(),
                exporter,
                [Fourcc::Xrgb8888],
                renderer.lock().unwrap().mem_formats().map(|code| Format {
                    code,
                    modifier: Modifier::Linear,
                }),
                Size::from((mode.size().0 as u32, mode.size().1 as u32)),
                Some(gbm.clone()),
            )?;

            compositors.insert(output.clone(), compositor);
        }
    }

    let compositors = Arc::new(Mutex::new(compositors));

    let drm_compositors = compositors.clone();
    let drm_renderer = renderer.clone();
    event_loop
        .handle()
        .insert_source(drm_notifier, move |event, _, state| match event {
            DrmEvent::VBlank(crtc) => {
                let mut compositors = drm_compositors.lock().unwrap();
                let mut renderer_guard = drm_renderer.lock().unwrap();
                let renderer = &mut *renderer_guard;
                for (output, compositor) in compositors.iter_mut() {
                    if compositor.crtc() == crtc {
                        let elements = space_render_elements::<GlesRenderer, Window, _>(
                            renderer,
                            [&state.space],
                            output,
                            1.0,
                        )
                        .expect("Output without mode");

                        let _ = compositor.render_frame(
                            renderer,
                            &elements,
                            Color32F::from([0.1, 0.1, 0.1, 1.0]),
                            FrameFlags::DEFAULT,
                        );
                        let _ = compositor.queue_frame(None::<()>);
                    }
                }
            }
            _ => {}
        })?;

    {
        let mut compositors = compositors.lock().unwrap();
        let mut renderer_guard = renderer.lock().unwrap();
        let renderer = &mut *renderer_guard;
        for (output, compositor) in compositors.iter_mut() {
            let elements = space_render_elements::<GlesRenderer, Window, _>(
                renderer,
                [&state.space],
                output,
                1.0,
            )
            .expect("Output without mode");

            let _ = compositor.render_frame(
                renderer,
                &elements,
                Color32F::from([0.1, 0.1, 0.1, 1.0]),
                FrameFlags::DEFAULT,
            );
            let _ = compositor.queue_frame(None::<()>);
        }
    }

    let udev_backend = UdevBackend::new(&session.seat())?;
    event_loop
        .handle()
        .insert_source(udev_backend, move |_, _, _| {})?;
    event_loop
        .handle()
        .insert_source(notifier, move |_, _, _| {})?;

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    let mut running = true;
    while running {
        if event_loop
            .dispatch(Some(Duration::from_millis(16)), &mut state)
            .is_err()
        {
            running = false;
        }
        state.space.refresh();
        state.popups.cleanup();
        let _ = state.display_handle.flush_clients();
    }

    Ok(())
}
