#![allow(warnings)]
use egui::{
    Align2, Color32, Context, FontData, FontDefinitions, FontFamily, FontId, FontTweak, Key,
    Modifiers, Pos2, Rect, RichText, ScrollArea, Slider, Stroke, TextureId, Vec2, Widget,
};
use egui_d3d11::DirectX11App;
use faithe::{internal::alloc_console, pattern::Pattern};
use std::{
    intrinsics::transmute,
    sync::{Arc, Once},
};
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Dxgi::{Common::DXGI_FORMAT, IDXGISwapChain},
        UI::WindowsAndMessaging::{CallWindowProcW, SetWindowLongPtrA, GWLP_WNDPROC, WNDPROC},
    },
};

#[no_mangle]
extern "stdcall" fn DllMain(hinst: usize, reason: u32) -> i32 {
    if reason == 1 {
        std::thread::spawn(move || unsafe { main_thread(hinst) });
    }

    1
}

static mut APP: DirectX11App<i32> = DirectX11App::new();
static mut OLD_WND_PROC: Option<WNDPROC> = None;

type FnPresent = unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32) -> HRESULT;
static mut O_PRESENT: Option<FnPresent> = None;

type FnResizeBuffers =
    unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;
static mut O_RESIZE_BUFFERS: Option<FnResizeBuffers> = None;

unsafe extern "stdcall" fn hk_present(
    swap_chain: IDXGISwapChain,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        APP.init_default(&swap_chain, ui);

        let desc = swap_chain.GetDesc().unwrap();
        if desc.OutputWindow.0 == -1 {
            panic!("Invalid window handle");
        }

        OLD_WND_PROC = Some(transmute(SetWindowLongPtrA(
            desc.OutputWindow,
            GWLP_WNDPROC,
            hk_wnd_proc as usize as _,
        )));
    });

    APP.present(&swap_chain);

    O_PRESENT.as_ref().unwrap()(swap_chain, sync_interval, flags)
}

unsafe extern "stdcall" fn hk_resize_buffers(
    swap_chain: IDXGISwapChain,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    swap_chain_flags: u32,
) -> HRESULT {
    eprintln!("Resizing buffers");

    APP.resize_buffers(&swap_chain, || {
        O_RESIZE_BUFFERS.as_ref().unwrap()(
            swap_chain.clone(),
            buffer_count,
            width,
            height,
            new_format,
            swap_chain_flags,
        )
    })
}

unsafe extern "stdcall" fn hk_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    APP.wnd_proc(msg, wparam, lparam);

    CallWindowProcW(OLD_WND_PROC.unwrap(), hwnd, msg, wparam, lparam)
}

static mut FRAME: i32 = 0;
fn ui(ctx: &Context, i: &mut i32) {
    unsafe {
        // You should not use statics like this, it's made
        // this way for the sake of example.
        static mut UI_CHECK: bool = true;
        static mut TEXT: Option<String> = None;
        static mut VALUE: f32 = 0.;
        static mut COLOR: [f32; 3] = [0., 0., 0.];
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            // Uncomment this to set other fonts.
            // let mut fonts = FontDefinitions::default();
            // let mut tweak = FontTweak::default();
            // fonts.font_data.insert(
            //     "my_font".to_owned(),
            //     FontData::from_static(include_bytes!("Lobster-Regular.ttf")).tweak(tweak),
            // );
            // fonts
            //     .families
            //     .get_mut(&FontFamily::Proportional)
            //     .unwrap()
            //     .insert(0, "my_font".to_owned());
            // fonts
            //     .families
            //     .get_mut(&FontFamily::Monospace)
            //     .unwrap()
            //     .push("my_font".to_owned());
            // ctx.set_fonts(fonts);
        });

        if TEXT.is_none() {
            TEXT = Some(String::from("Test"));
        }

        ctx.debug_painter().text(
            Pos2::new(0., 0.),
            Align2::LEFT_TOP,
            "Bruh",
            FontId::default(),
            Color32::RED,
        );

        egui::containers::Window::new("Main menu").show(ctx, |ui| {
            ui.label(RichText::new("Test").color(Color32::BLACK));
            ui.label(RichText::new("Other").color(Color32::WHITE));
            ui.separator();

            ui.label(RichText::new(format!("I: {}", *i)).color(Color32::LIGHT_RED));

            let input = ctx.input().pointer.clone();
            ui.label(format!(
                "X1: {} X2: {}",
                input.button_down(egui::PointerButton::Extra1),
                input.button_down(egui::PointerButton::Extra2)
            ));

            let mods = ui.input().modifiers;
            ui.label(format!(
                "Ctrl: {} Shift: {} Alt: {}",
                mods.ctrl, mods.shift, mods.alt
            ));

            if ui.input().modifiers.matches(Modifiers::CTRL) && ui.input().key_pressed(Key::R) {
                println!("Pressed");
            }

            unsafe {
                ui.checkbox(&mut UI_CHECK, "Some checkbox");
                ui.text_edit_singleline(TEXT.as_mut().unwrap());
                ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                    for i in 1..=100 {
                        ui.label(format!("Label: {}", i));
                    }
                });

                Slider::new(&mut VALUE, -1.0..=1.0).ui(ui);

                ui.color_edit_button_rgb(&mut COLOR);
            }

            ui.label(format!(
                "{:?}",
                &ui.input().pointer.button_down(egui::PointerButton::Primary)
            ));
            if ui.button("You can't click me yet").clicked() {
                *i += 1;
            }
        });

        egui::Window::new("Image").show(ctx, |ui| {
            unsafe {
                static mut IMG: TextureId = TextureId::Managed(0);

                if IMG == TextureId::Managed(0) {
                    let tex = Box::leak(Box::new(ctx.load_texture(
                        "logo",
                        egui_extras::image::load_image_bytes(include_bytes!("../../logo.bmp")).unwrap(),
                        egui::TextureFilter::Linear
                    )));

                    IMG = tex.id();
                }

                ui.image(IMG, Vec2::new(500., 391.));
            }
        });

        ctx.debug_painter().rect(
            Rect {
                min: Pos2::new(200.0, 200.0),
                max: Pos2::new(250.0, 250.0),
            },
            10.0,
            Color32::from_rgba_premultiplied(255, 0, 0, 150),
            Stroke::none(),
        );

        ctx.debug_painter().circle(
            Pos2::new(350.0, 350.0),
            35.0,
            Color32::from_rgba_premultiplied(0, 255, 0, 200),
            Stroke::none(),
        );
    }
}

unsafe fn main_thread(_hinst: usize) {
    alloc_console().unwrap();

    eprintln!("Hello World!");

    let present = faithe::internal::find_pattern(
        "gameoverlayrenderer64.dll",
        Pattern::from_ida_style("48 89 6C 24 18 48 89 74 24 20 41 56 48 83 EC 20 41"),
    )
    .unwrap_or_else(|_| {
        faithe::internal::find_pattern(
            "dxgi.dll",
            Pattern::from_ida_style("48 89 5C 24 10 48 89 74 24 20 55 57 41 56"),
        )
        .unwrap()
    })
    .unwrap() as usize;

    eprintln!("Present: {:X}", present);

    let swap_buffers = faithe::internal::find_pattern(
        "gameoverlayrenderer64.dll",
        Pattern::from_ida_style(
            "48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 41 56 41 57 48 83 EC 30 44",
        ),
    )
    .unwrap_or_else(|_| {
        faithe::internal::find_pattern(
            "dxgi.dll",
            Pattern::from_ida_style("48 8B C4 55 41 54 41 55 41 56 41 57 48 8D 68 B1 48 81 EC C0"),
        )
        .unwrap()
    })
    .unwrap() as usize;

    eprintln!("Buffers: {:X}", swap_buffers);

    sunshine::create_hook(
        sunshine::HookType::Compact,
        transmute::<_, FnPresent>(present),
        hk_present as FnPresent,
        &mut O_PRESENT,
    )
    .unwrap();

    sunshine::create_hook(
        sunshine::HookType::Compact,
        transmute::<_, FnResizeBuffers>(swap_buffers),
        hk_resize_buffers as FnResizeBuffers,
        &mut O_RESIZE_BUFFERS,
    )
    .unwrap();

    #[allow(clippy::empty_loop)]
    loop {}
}

// for<'r, 's> fn(&'r egui::Context, &'s mut i32) -> _
// for<'r, 's> fn(&'r egui::context::Context, &'s mut _) -> _
