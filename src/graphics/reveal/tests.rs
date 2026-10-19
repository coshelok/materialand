use super::*;

#[test]
fn reveal_coordinates_follow_scaled_and_clipped_textures() {
    let texture = Size::from((200, 100));
    let whole = Rectangle::from_size((200.0, 100.0).into());
    let scaled = Rectangle::new((30, 50).into(), (100, 50).into());
    assert_eq!(
        screen_mapping(texture, whole, scaled),
        [100.0, 50.0, 30.0, 50.0]
    );
    let clipped = Rectangle::new((40.0, 20.0).into(), (120.0, 60.0).into());
    let shown = Rectangle::new((50, 60).into(), (60, 30).into());
    // Clipping changes the sampled region, never the screen-space circle.
    assert_eq!(
        screen_mapping(texture, clipped, shown),
        [100.0, 50.0, 30.0, 50.0]
    );
}

#[test]
#[ignore = "requires an EGL device (hardware or Mesa software); run explicitly"]
fn egl_reveal_preserves_pixels_clipping_scaling_and_shader_isolation() {
    use smithay::backend::{
        allocator::Fourcc,
        egl::{EGLContext, EGLDevice, EGLDisplay},
        renderer::{
            element::{
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                Kind,
            },
            gles::GlesTexture,
            Bind, Color32F, ExportMem, Frame, Offscreen, Renderer,
        },
    };
    let device = EGLDevice::enumerate()
        .expect("enumerate EGL devices")
        .next()
        .expect("EGL device");
    // Smithay exclusively owns this display and renderer on this test thread.
    let display = unsafe { EGLDisplay::new(device) }.unwrap();
    let context = EGLContext::new(&display).unwrap();
    let mut renderer = unsafe { GlesRenderer::new(context) }.unwrap();
    let shader = program(&mut renderer).expect("compile reveal shader");
    let source_size = (64, 32);
    let buffer = MemoryRenderBuffer::from_slice(
        &[255, 0, 0, 255].repeat(64 * 32),
        Fourcc::Abgr8888,
        source_size,
        1,
        Transform::Normal,
        None,
    );
    let plain_buffer = MemoryRenderBuffer::from_slice(
        &[0, 255, 0, 255].repeat(8 * 8),
        Fourcc::Abgr8888,
        (8, 8),
        1,
        Transform::Normal,
        None,
    );
    let size = Size::<i32, Physical>::from((96, 64));
    let mut texture: GlesTexture = renderer
        .create_buffer(Fourcc::Abgr8888, Size::from((size.w, size.h)))
        .unwrap();
    let mut layers = Vec::new();
    crate::graphics::uploads::push_layer(
        &mut layers,
        &mut renderer,
        &crate::shell::LayerFrame {
            buffer: buffer.clone(),
            loc: (12.0, 10.0).into(),
            size: Some((32, 16).into()),
        },
        Some(1.0),
        Some(CircleMask {
            cx: 28.0,
            cy: 18.0,
            radius: 6.0,
        }),
    );
    assert_eq!(layers[0].src().size, (64.0, 32.0).into());
    assert_eq!(layers[0].geometry(1.0.into()).size, (32, 16).into());
    for transform in [Transform::Normal, Transform::Flipped180] {
        for (source, destination) in [
            (
                Rectangle::from_size((64.0, 32.0).into()),
                Rectangle::new((12, 10).into(), (64, 32).into()),
            ),
            (
                Rectangle::from_size((64.0, 32.0).into()),
                Rectangle::new((12, 10).into(), (32, 16).into()),
            ),
            (
                Rectangle::new((16.0, 0.0).into(), (32.0, 32.0).into()),
                Rectangle::new((20, 10).into(), (16, 16).into()),
            ),
        ] {
            let inner = MemoryRenderBufferRenderElement::from_buffer(
                &mut renderer,
                (0.0, 0.0),
                &buffer,
                Some(0.5),
                None,
                None,
                Kind::Unspecified,
            )
            .unwrap();
            let element = RevealElement::new(
                inner,
                shader.clone(),
                CircleMask {
                    cx: 28.0,
                    cy: 18.0,
                    radius: 6.0,
                },
                source_size.into(),
            );
            assert!(element.opaque_regions(1.0.into()).is_empty());
            assert!(element.underlying_storage(&mut renderer).is_none());
            let plain = MemoryRenderBufferRenderElement::from_buffer(
                &mut renderer,
                (80.0, 0.0),
                &plain_buffer,
                None,
                None,
                None,
                Kind::Unspecified,
            )
            .unwrap();
            let mut target = renderer.bind(&mut texture).unwrap();
            let mut frame = renderer.render(&mut target, size, transform).unwrap();
            frame
                .clear(
                    Color32F::new(0.0, 0.0, 1.0, 1.0),
                    &[Rectangle::from_size(size)],
                )
                .unwrap();
            element
                .draw(
                    &mut frame,
                    source,
                    destination,
                    &[Rectangle::from_size(destination.size)],
                    &[],
                )
                .unwrap();
            plain
                .draw(
                    &mut frame,
                    plain.src(),
                    plain.geometry(1.0.into()),
                    &[Rectangle::from_size((8, 8).into())],
                    &[],
                )
                .unwrap();
            frame.finish().unwrap().wait().unwrap();
            let mapping = renderer
                .copy_framebuffer(
                    &target,
                    Rectangle::from_size(Size::from((size.w, size.h))),
                    Fourcc::Abgr8888,
                )
                .unwrap();
            let data = renderer.map_texture(&mapping).unwrap();
            let pixel = |x: usize, y: usize| {
                // Smithay Normal maps Y directly into the offscreen GL target;
                // Flipped180 (the nested backend) reverses its rows.
                let row = if transform == Transform::Normal {
                    y
                } else {
                    63 - y
                };
                &data[(row * 96 + x) * 4..(row * 96 + x) * 4 + 4]
            };
            let center = pixel(28, 18);
            assert!(
                (i16::from(center[0]) - 128).abs() <= 1
                    && center[1] == 0
                    && (i16::from(center[2]) - 127).abs() <= 1,
                "center {center:?}, transform {transform:?}"
            );
            assert_eq!(pixel(21, 11), [0, 0, 255, 255], "outside circle");
            assert_eq!(
                pixel(82, 2),
                [0, 255, 0, 255],
                "shader must not leak into subsequent elements"
            );
        }
    }
}
