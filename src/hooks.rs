use windows::{Win32::Graphics::Dxgi::IDXGISwapChain, core::HRESULT};
retour::static_detour! {
    pub static present_hook: unsafe extern "system" fn(IDXGISwapChain, u32, u32) -> HRESULT;
}
