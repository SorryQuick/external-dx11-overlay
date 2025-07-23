use windows::{
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::{
                D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11CreateDeviceAndSwapChain,
                ID3D11Device, ID3D11DeviceContext,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
                    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_SAMPLE_DESC,
                },
                DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
                IDXGISwapChain,
            },
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
            RegisterClassExW, UnregisterClassW, WNDCLASSEXW, WS_EX_OVERLAPPEDWINDOW,
            WS_OVERLAPPEDWINDOW,
        },
    },
    core::{Interface, w},
};

/*
 *
 *    AddressFinder contains the necessary utilities to find addresses based on a given pattern.
 *    It's not particularly fast, but generally only needs to run once at the beginning and can be
 *    done in another thread if necessary. It also contains a utility to find the address of
 *    DirectX's present. This only works with DirectX11, but can easily be modified to work with
 *    another version. Functions are very primitive and return raw usize pointers. PLEASE USE
 *    CAUTION AND VERIFIY THOSE POINTERS ARE NOT ZERO. There is no point in changing this to return
 *    rust-safe types, as the returned pointers will most definitely be used in very unsafe ways.
 *
 *
 * */

pub struct AddressFinder {
    pub base_addr: usize,
    pub module_size: usize,
}

impl AddressFinder {
    /* pub fn find_addr_templateonly(self: &AddressFinder) -> usize {
        /*0x1410ca370*/
        /*48 89 5C 24 08 55 56 57 41 54 41 55 41 56 41 57 48 8B EC 48 83 EC 70 8B 82 A0*/
        let pattern: Vec<u8> = vec![
            0x48, 0x89, 0x5c, 0x24, 0x08, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56,
            0x41, 0x57, 0x48, 0x8b, 0xec, 0x48, 0x83, 0xec, 0x70, 0x8b, 0x82, 0xa0,
        ];
        let pattern_wildcards: Vec<usize> = vec![];

        return self.find_pattern_addr(pattern, pattern_wildcards, "template_hook");
    }*/

    //Uses a KMP algorithm
    #[allow(dead_code)]
    fn find_pattern_addr(self: &AddressFinder, pat: Vec<u8>, wildcards: Vec<usize>) -> usize {
        let base = self.base_addr;
        let m: usize = pat.len();
        let n: usize = self.module_size;

        let mut lps = vec![0; m];

        compute_lps_array(&pat, m, &mut lps);

        let mut i: usize = 0; // offset from base
        let mut j: usize = 0; // pattern offset
        while (n - i) >= (m - j) {
            let addr: usize = base + i;
            let byte_to_check: u8 = unsafe { *(addr as *const u8) };
            let pattern_byte: u8 = pat[j];

            if pattern_byte == byte_to_check || wildcards.contains(&j) {
                j += 1;
                i += 1;
                if j == m {
                    return base + i - j;
                }
            } else {
                if j != 0 {
                    j = lps[j - 1];
                } else {
                    i = i + 1;
                }
            }
        }
        return 0;
    }

    #[allow(dead_code)]
    pub fn find_addr_present(self: &AddressFinder) -> usize {
        let mut p_device: Option<ID3D11Device> = None;
        let mut p_context: Option<ID3D11DeviceContext> = None;
        let mut p_swap_chain: Option<IDXGISwapChain> = None;

        let classname = w!("external_dx11_overlay");

        unsafe extern "system" fn wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        let module_handle = unsafe { GetModuleHandleW(None) };
        if module_handle.is_err() {
            return 0;
        }

        let window_class: WNDCLASSEXW = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: module_handle.unwrap().into(),
            lpszClassName: classname,
            ..Default::default()
        };
        let registered_window_class = unsafe { RegisterClassExW(&window_class) };

        if registered_window_class == 0 {
            return 0;
        }

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_OVERLAPPEDWINDOW,
                classname,
                classname,
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                100,
                100,
                None,
                None,
                window_class.hInstance,
                None,
            )
        };

        if hwnd == HWND(0) {
            let _ = unsafe { UnregisterClassW(classname, window_class.hInstance) };
            return 0;
        }

        let swapchain_desc = DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC {
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
                ..Default::default()
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 1,
            OutputWindow: hwnd,
            Windowed: BOOL(1),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let featurelevels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0];
        unsafe {
            let _ = D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&featurelevels),
                D3D11_SDK_VERSION,
                Some(&swapchain_desc),
                Some(&mut p_swap_chain),
                Some(&mut p_device),
                None,
                Some(&mut p_context),
            );
        };

        if p_swap_chain.is_none() {
            let _ = unsafe { DestroyWindow(hwnd) };
            let _ = unsafe { UnregisterClassW(classname, window_class.hInstance) };
            return 0;
        }

        let swapchain = p_swap_chain.unwrap();
        let present_addr = swapchain.vtable().Present as usize;

        unsafe {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(classname, window_class.hInstance);
        }
        present_addr
    }
}

#[allow(dead_code)]
fn compute_lps_array(pat: &Vec<u8>, m: usize, lps: &mut Vec<usize>) {
    let mut len = 0;
    lps[0] = 0;
    let mut i = 1;
    while i < m {
        if pat[i] == pat[len] {
            len += 1;
            lps[i] = len;
            i += 1;
        } else {
            if len != 0 {
                len = lps[len - 1];
            } else {
                lps[i] = 0;
                i += 1;
            }
        }
    }
}
