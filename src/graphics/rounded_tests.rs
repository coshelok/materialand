use super::*;

#[test]
#[ignore = "requires an EGL device (hardware or Mesa software); run explicitly"]
fn egl_rounded_previews_preserve_clipping_transforms_and_shader_isolation() {
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
    let shader = program(&mut renderer).expect("compile rounded preview shader");
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
            let element = RoundedElement::new(
                inner,
                Some(shader.clone()),
                RectF {
                    x: 20.0,
                    y: 10.0,
                    w: 24.0,
                    h: 16.0,
                },
                6.0,
                size,
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
            assert_eq!(pixel(20, 10), [0, 0, 255, 255], "outside rounded corner");
            assert_eq!(
                pixel(82, 2),
                [0, 255, 0, 255],
                "shader must not leak into subsequent elements"
            );
        }
    }
}
