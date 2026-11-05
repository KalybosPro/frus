// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **Which GPU interface a page draws through** (milestone 745): WebGPU where the browser
//! offers it, WebGL 2 where it does not.
//!
//! A browser can lack WebGPU in three ways: no `navigator.gpu` at all (Chrome on Android
//! before Android 12, older Safari and Firefox), an API with no adapter behind it (a GPU or
//! driver the browser has blocked), or an adapter that cannot make a device. The choice is
//! made **before the canvas is given a context**: a canvas that has handed out a `webgpu`
//! context never hands out a `webgl2` one, so the WebGPU path is only taken once a device
//! has been made, and the surface comes last.

/// What the browser's WebGPU turned out to offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WebGpu {
    /// WebGPU was not among the backends asked for: it was not probed.
    NotAsked,
    /// No `navigator.gpu`: the browser has no WebGPU (or not on this system).
    NoApi,
    /// `navigator.gpu` is there, but the browser gives no adapter: typically a GPU or a
    /// driver it has blocked.
    NoAdapter,
    /// An adapter, but no device could be made from it: why.
    NoDevice(String),
    /// An adapter and a device: WebGPU is used.
    Ready,
}

impl std::fmt::Display for WebGpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebGpu::NotAsked => f.write_str("WebGPU was not asked for"),
            WebGpu::NoApi => f.write_str("this browser has no WebGPU (no navigator.gpu)"),
            WebGpu::NoAdapter => {
                f.write_str("this browser's WebGPU gives no adapter (a blocked GPU or driver)")
            }
            WebGpu::NoDevice(why) => write!(f, "this browser's WebGPU made no device: {why}"),
            WebGpu::Ready => f.write_str("WebGPU is available"),
        }
    }
}

/// The backend a page draws through, from the backends asked for and what WebGPU offered:
/// WebGPU when it is ready, WebGL 2 (`GL`) otherwise, and an error naming both when neither
/// can be had.
pub(crate) fn choose(requested: wgpu::Backends, webgpu: &WebGpu) -> Result<wgpu::Backends, String> {
    if requested.contains(wgpu::Backends::BROWSER_WEBGPU) && *webgpu == WebGpu::Ready {
        return Ok(wgpu::Backends::BROWSER_WEBGPU);
    }
    if requested.contains(wgpu::Backends::GL) {
        return Ok(wgpu::Backends::GL);
    }
    Err(match webgpu {
        WebGpu::NotAsked => format!("no browser GPU interface among the backends {requested:?}"),
        other => format!("{other}, and WebGL 2 was not asked for"),
    })
}

/// The backends a page asks for: WebGPU first, WebGL 2 behind it.
#[cfg(target_arch = "wasm32")]
pub(crate) const REQUESTED: wgpu::Backends =
    wgpu::Backends::BROWSER_WEBGPU.union(wgpu::Backends::GL);

/// A device made through WebGPU before any surface: what the renderer goes on with.
#[cfg(target_arch = "wasm32")]
pub(crate) struct Opened {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// The instance a page draws through, and — on WebGPU — the adapter and device already made.
/// On WebGL 2 the adapter comes from the canvas's context, so the caller makes the surface
/// first and asks for the adapter with it.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn open(
    requested: wgpu::Backends,
) -> anyhow::Result<(wgpu::Instance, Option<Opened>)> {
    let instance = |backends| {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
    };
    let mut probe = WebGpu::NotAsked;
    if requested.contains(wgpu::Backends::BROWSER_WEBGPU) {
        probe = if !has_navigator_gpu() {
            WebGpu::NoApi
        } else {
            // Not before `navigator.gpu` is known to be there: without it, the instance would
            // be wgpu's own, which knows no WebGPU.
            let webgpu = instance(wgpu::Backends::BROWSER_WEBGPU);
            match webgpu
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
            {
                Err(_) => WebGpu::NoAdapter,
                Ok(adapter) => match crate::renderer::request_device(&adapter).await {
                    Err(err) => WebGpu::NoDevice(format!("{err:#}")),
                    Ok((device, queue)) => {
                        log::info!("GPU interface: WebGPU");
                        return Ok((
                            webgpu,
                            Some(Opened {
                                adapter,
                                device,
                                queue,
                            }),
                        ));
                    }
                },
            }
        };
    }
    let backends = choose(requested, &probe).map_err(anyhow::Error::msg)?;
    log::info!("GPU interface: WebGL 2 ({probe})");
    Ok((instance(backends), None))
}

/// Whether the page's global (a window or a worker) has a `navigator.gpu`.
#[cfg(target_arch = "wasm32")]
fn has_navigator_gpu() -> bool {
    use wasm_bindgen::JsValue;
    let navigator = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("navigator"))
        .unwrap_or(JsValue::UNDEFINED);
    if navigator.is_undefined() || navigator.is_null() {
        return false;
    }
    js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .map(|gpu| !gpu.is_undefined() && !gpu.is_null())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::Backends;

    const BOTH: Backends = Backends::BROWSER_WEBGPU.union(Backends::GL);

    #[test]
    fn webgpu_is_used_when_it_is_ready() {
        assert_eq!(choose(BOTH, &WebGpu::Ready), Ok(Backends::BROWSER_WEBGPU));
    }

    /// No API (an Android phone before Android 12), no adapter (a blocked GPU) or no device:
    /// WebGL 2 every time.
    #[test]
    fn webgl2_is_used_whenever_webgpu_is_missing() {
        for missing in [
            WebGpu::NoApi,
            WebGpu::NoAdapter,
            WebGpu::NoDevice("limits".into()),
            WebGpu::NotAsked,
        ] {
            assert_eq!(choose(BOTH, &missing), Ok(Backends::GL), "{missing}");
        }
        assert_eq!(choose(Backends::GL, &WebGpu::NotAsked), Ok(Backends::GL));
    }

    /// WebGL 2 alone does not take a WebGPU that is ready: the backends asked for decide.
    #[test]
    fn the_backends_asked_for_decide() {
        assert_eq!(choose(Backends::GL, &WebGpu::Ready), Ok(Backends::GL));
    }

    #[test]
    fn neither_is_an_error_that_says_why() {
        let err = choose(Backends::BROWSER_WEBGPU, &WebGpu::NoAdapter).unwrap_err();
        assert!(err.contains("no adapter"), "{err}");
        assert!(err.contains("WebGL 2"), "{err}");
        assert!(choose(Backends::VULKAN, &WebGpu::NotAsked).is_err());
    }
}

/// What WebGL 2 asks of every shader frus draws with (milestone 745), checked on the shaders'
/// own source, since no test host here has WebGL: no storage buffers, no compute stage, and
/// uniforms whose size is a multiple of 16 bytes — WebGL 2 binds no other (a `vec2` viewport
/// alone failed every pipeline at the start).
#[cfg(test)]
mod webgl2_shaders {
    use wgpu::naga;

    const SHADERS: [(&str, &str); 6] = [
        ("composite", include_str!("shaders/composite.wgsl")),
        ("filter", include_str!("shaders/filter.wgsl")),
        ("image", include_str!("shaders/image.wgsl")),
        ("mipmap", include_str!("shaders/mipmap.wgsl")),
        ("path", include_str!("shaders/path.wgsl")),
        ("quad", include_str!("shaders/quad.wgsl")),
    ];

    #[test]
    fn every_shader_fits_webgl2() {
        for (name, source) in SHADERS {
            let module = naga::front::wgsl::parse_str(source)
                .unwrap_or_else(|err| panic!("{name}: {}", err.emit_to_string(source)));
            for entry in &module.entry_points {
                assert_ne!(
                    entry.stage,
                    naga::ShaderStage::Compute,
                    "{name}: WebGL 2 has no compute"
                );
            }
            for (_, global) in module.global_variables.iter() {
                let label = global.name.as_deref().unwrap_or("?");
                assert!(
                    !matches!(global.space, naga::AddressSpace::Storage { .. }),
                    "{name}: `{label}` is a storage buffer, which WebGL 2 has not"
                );
                if global.space == naga::AddressSpace::Uniform {
                    let size = module.types[global.ty].inner.size(module.to_ctx());
                    assert_eq!(
                        size % 16,
                        0,
                        "{name}: the uniform `{label}` is {size} bytes, not a multiple of 16"
                    );
                }
            }
        }
    }
}
