use std::{
    sync::{Mutex, atomic::Ordering},
    time::Instant,
};

use windows::{
    Win32::{
        Foundation::BOOL,
        Graphics::{
            Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            Direct3D11::{
                D3D11_BIND_SHADER_RESOURCE, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA,
                D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD, D3D11_BLEND_SRC_ALPHA, D3D11_BLEND_ZERO,
                D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_NEVER, D3D11_CPU_ACCESS_WRITE,
                D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_FLOAT32_MAX, D3D11_MAP_WRITE_DISCARD,
                D3D11_MAPPED_SUBRESOURCE, D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_CLAMP,
                D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC, D3D11_VIEWPORT, ID3D11BlendState,
                ID3D11Device, ID3D11DeviceContext, ID3D11PixelShader, ID3D11RenderTargetView,
                ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
            },
            Dxgi::{
                Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
                IDXGISwapChain,
            },
        },
    },
    core::{Error, HRESULT},
};

use crate::{
    debug::{
        DEBUG_FEATURES,
        debug_overlay::draw_debug_overlay,
        statistics::{self, send_statistic},
    },
    hooks::present_hook,
};

use super::{FRAME_BUFFER, OVERLAY_STATE};

//Ultra basic shader.
//Have to be compiled on windows with fxc.
static VS_OVERLAY: &[u8] = include_bytes!("vs.cso");
static PS_OVERLAY: &[u8] = include_bytes!("ps.cso");

//Contains DirectX related stuff that can be reused over many frames.
pub struct OverlayState {
    pub width: u32,
    pub height: u32,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    overlay_texture: ID3D11Texture2D,
    shader_resource_view: ID3D11ShaderResourceView,
    blend_state: ID3D11BlendState,
    sampler_state: ID3D11SamplerState,
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    target_view: Option<ID3D11RenderTargetView>,
    viewport: D3D11_VIEWPORT,
    blend_factor: [f32; 4],
}

impl OverlayState {
    pub fn recreate(
        &mut self,
        swapchain: &IDXGISwapChain,
        new_width: u32,
        new_height: u32,
    ) -> (u32, u32) {
        let (txt, shd, viewport) =
            create_overlay_texture_and_srv(&self.device, new_width, new_height).unwrap();
        self.overlay_texture = txt;
        self.shader_resource_view = shd;
        self.viewport = viewport;

        self.width = new_width;
        self.height = new_height;

        if let Ok(rtv) = create_target_view(swapchain, &self.device) {
            self.target_view = Some(rtv);
        }
        (new_width, new_height)
    }
}

///This is our big present hook. Takes a the latest frame sent from Blish in FRAME_BUFFER.
pub fn detoured_present(swapchain: IDXGISwapChain, sync_interval: u32, flags: u32) -> HRESULT {
    let start = Instant::now();
    //Macro to make it less ugly to return early.
    macro_rules! return_present {
        () => {
            return present_hook.call(swapchain, sync_interval, flags)
        };
    }
    if !DEBUG_FEATURES.rendering_enabled.load(Ordering::Relaxed) {
        unsafe { return_present!() }
    }
    unsafe {
        if OVERLAY_STATE.get().is_none() {
            initialize_overlay_state(&swapchain);
        }

        //Check if we need to cache stuff over again
        let mut lock = OVERLAY_STATE.get().unwrap().lock().unwrap();
        let recreate = if let Some(state) = lock.as_ref() {
            state.device.GetDeviceRemovedReason().is_err()
        } else {
            true
        };
        if recreate {
            drop(lock);
            initialize_overlay_state(&swapchain);
            lock = OVERLAY_STATE.get().unwrap().lock().unwrap();
        }

        let state = lock.as_mut().unwrap();

        //Get newly-received width from blish
        let (new_width, new_height) = if let Some(frame_lock) = FRAME_BUFFER.get() {
            let frame = frame_lock.lock().unwrap();
            if frame.width == 0 || frame.height == 0 {
                return_present!();
            }
            (frame.width, frame.height)
        } else {
            return_present!();
        };

        let mut width = state.width;
        let mut height = state.height;

        //Checks if resolution changed. If so, update the state.
        if new_width != state.width || new_height != state.height {
            (width, height) = state.recreate(&swapchain, new_width, new_height);
        }

        let ctx = &state.context;

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if let Err(e) = ctx.Map(
            &state.overlay_texture,
            0,
            D3D11_MAP_WRITE_DISCARD,
            0,
            Some(&mut mapped),
        ) {
            log::error!("Error mapping texture: {}", e.to_string());
            return_present!();
        }

        if let Err(_) = copy_frame_into_map(width as usize, height as usize, &mapped) {
            log::error!("Could not copy frame into map. ",);
            ctx.Unmap(&state.overlay_texture, 0);
            return_present!();
        }

        ctx.Unmap(&state.overlay_texture, 0);

        ctx.OMSetBlendState(&state.blend_state, Some(&state.blend_factor), 0xffffffff);

        if state.target_view.is_none() {
            if let Ok(trv) = create_target_view(&swapchain, &state.device) {
                state.target_view = Some(trv);
            }
        }
        if state.target_view.is_some() {
            ctx.OMSetRenderTargets(Some(&[state.target_view.clone()]), None);
        }

        //Viewport
        ctx.RSSetViewports(Some(&[state.viewport]));

        //Shaders
        ctx.VSSetShader(&state.vertex_shader, None);
        ctx.PSSetShader(&state.pixel_shader, None);

        // Bind SRV and sampler
        ctx.PSSetShaderResources(0, Some(&[Some(state.shader_resource_view.clone())]));
        ctx.PSSetSamplers(0, Some(&[Some(state.sampler_state.clone())]));

        // Draw full-screen triangle
        ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        ctx.Draw(3, 0);

        //Stats
        let frame_time_custom = start.elapsed().as_nanos() as u32;
        send_statistic(statistics::debug_stat::FRAME_TIME_CUSTOM, frame_time_custom);

        //Original present
        let result = present_hook.call(swapchain, sync_interval, flags);

        let frame_time_total = start.elapsed().as_nanos() as u32;
        send_statistic(statistics::debug_stat::FRAME_TIME_TOTAL, frame_time_total);
        send_statistic(
            statistics::debug_stat::FRAME_TIME_DIFF,
            frame_time_total - frame_time_custom,
        );
        return result;
    }
}

fn get_device_and_context(
    swapchain: &IDXGISwapChain,
) -> Result<(ID3D11Device, ID3D11DeviceContext), ()> {
    unsafe {
        if let Ok(device) = swapchain.GetDevice::<ID3D11Device>() {
            if let Ok(ctx) = (device).GetImmediateContext() {
                return Ok((device, ctx));
            }
        }
    }
    Err(())
}
fn create_target_view(
    swapchain: &IDXGISwapChain,
    device: &ID3D11Device,
) -> Result<ID3D11RenderTargetView, ()> {
    unsafe {
        if let Ok(buf) = swapchain.GetBuffer::<ID3D11Texture2D>(0) {
            let mut rtv: Option<ID3D11RenderTargetView> = None;
            if device
                .CreateRenderTargetView(
                    &buf,
                    None,
                    Some(&mut rtv as *mut Option<ID3D11RenderTargetView>),
                )
                .is_err()
            {
                log::error!("Could not render target view.");
                return Err(());
            }
            return Ok(rtv.unwrap());
        } else {
            log::error!("GetBuffer failed.");
            return Err(());
        }
    }
}

fn initialize_overlay_state(swapchain: &IDXGISwapChain) {
    let (device, context) =
        get_device_and_context(swapchain).expect("Could not get device and context from swapchain");
    let (txt, shd, viewport) = create_overlay_texture_and_srv(&device, 1920, 1080).unwrap();
    let state = OverlayState {
        width: 1920,
        height: 1080,
        device: device.clone(),
        context: context.clone(),
        overlay_texture: txt,
        shader_resource_view: shd,
        blend_state: create_blend_state(&device).unwrap(),
        sampler_state: create_sampler_state(&device).unwrap(),
        vertex_shader: create_vertex_shader(&device).unwrap(),
        pixel_shader: create_pixel_shader(&device).unwrap(),
        target_view: None,
        viewport,
        blend_factor: [0.0f32, 0.0f32, 0.0f32, 0.0f32],
    };
    let overlay_state = OVERLAY_STATE.get_or_init(|| Mutex::new(None));
    if let Ok(mut lock) = overlay_state.lock() {
        *lock = Some(state);
    } else {
        log::error!("Poisoned OVERLAY_STATE mutex.");
    }
}

///This function copies the data from FRAME_BUFFER into mapped data
fn copy_frame_into_map(
    width: usize,
    height: usize,
    mapped: &D3D11_MAPPED_SUBRESOURCE,
) -> Result<(), ()> {
    let frame_lock = FRAME_BUFFER.get().unwrap();
    let mut frame = frame_lock.lock().unwrap();

    //Draw the debug overlay if it's enabled. It's rather slow, but we don't care
    //since it's for debugging only anyway.
    if DEBUG_FEATURES.debug_overlay_enabled.load(Ordering::Relaxed) {
        draw_debug_overlay(&mut frame.pixels, width as u32);
    }

    //Copy to GPU
    for y in 0..height as usize {
        unsafe {
            let src_row = frame.pixels.as_ptr().add(y * width as usize * 4);
            let dst_row = (mapped.pData as *mut u8).add(y * mapped.RowPitch as usize);
            std::ptr::copy_nonoverlapping(src_row, dst_row, width as usize * 4);
        };
    }
    Ok(())
}

///Creates the Texture2D for a given dimension.
///It will be reused for as long as the resolution doesn't change.
pub fn create_overlay_texture(
    device: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<ID3D11Texture2D, Error> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DYNAMIC,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
        MiscFlags: 0,
    };

    let mut texture: Option<ID3D11Texture2D> = None;
    unsafe {
        device.CreateTexture2D(&desc, None, Some(&mut texture as *mut _))?;
    }

    Ok(texture.unwrap())
}

///Creates the vertex shader to be used to display the overlay. Will be reused forever.
pub fn create_vertex_shader(device: &ID3D11Device) -> Result<ID3D11VertexShader, Error> {
    let mut vs: Option<ID3D11VertexShader> = None;
    unsafe {
        device.CreateVertexShader(VS_OVERLAY, None, Some(&mut vs))?;
    }
    Ok(vs.unwrap())
}

///Creates the pixel shader to be used to display the overlay. Will be reused forever.
pub fn create_pixel_shader(device: &ID3D11Device) -> Result<ID3D11PixelShader, Error> {
    let mut ps: Option<ID3D11PixelShader> = None;
    unsafe {
        device.CreatePixelShader(PS_OVERLAY, None, Some(&mut ps))?;
    }
    Ok(ps.unwrap())
}

///Creates the SamplerState to be used to display the overlay. Will be reused forever.
//TODO: Verify this, my DirectX knowledge is not great.
pub fn create_sampler_state(device: &ID3D11Device) -> Result<ID3D11SamplerState, Error> {
    let sampler_desc = D3D11_SAMPLER_DESC {
        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
        AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
        ComparisonFunc: D3D11_COMPARISON_NEVER,
        MinLOD: 0.0,
        MaxLOD: D3D11_FLOAT32_MAX,
        ..Default::default()
    };

    let mut sampler: Option<ID3D11SamplerState> = None;
    unsafe {
        device.CreateSamplerState(&sampler_desc, Some(&mut sampler))?;
    }

    Ok(sampler.unwrap())
}

///Creates the Texture2D and ShaderResourceView to be used to display the overlay.
///Will be rerun every time the resolution changes.
pub fn create_overlay_texture_and_srv(
    device: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<(ID3D11Texture2D, ID3D11ShaderResourceView, D3D11_VIEWPORT), Error> {
    let texture = create_overlay_texture(device, width, height)?;

    let viewport = D3D11_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: width as f32,
        Height: height as f32,
        MinDepth: 0.0,
        MaxDepth: 1.0,
    };

    let mut srv: Option<ID3D11ShaderResourceView> = None;
    unsafe {
        device.CreateShaderResourceView(&texture, None, Some(&mut srv))?;
    }

    Ok((texture, srv.unwrap(), viewport))
}

///Creates the BlendState to be used to display the overlay. Will be reused forever.
//TODO: Verify this, my DirectX knowledge is not great.
pub fn create_blend_state(device: &ID3D11Device) -> Result<ID3D11BlendState, Error> {
    let mut blend_desc = D3D11_BLEND_DESC::default();

    blend_desc.RenderTarget[0].BlendEnable = BOOL(1);
    blend_desc.RenderTarget[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
    blend_desc.RenderTarget[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
    blend_desc.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
    blend_desc.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_ONE;
    blend_desc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ZERO;
    blend_desc.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
    blend_desc.RenderTarget[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8;

    let mut blend_state: Option<ID3D11BlendState> = None;
    unsafe {
        device.CreateBlendState(&blend_desc, Some(&mut blend_state))?;
    }

    Ok(blend_state.unwrap())
}
