use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use args::Args;
use chip8::{Chip8, KeyMask};
use sdl2::audio::AudioCallback;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::Sdl;

mod args;
mod chip8;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 32;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn to_color(hex: String) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    Color::RGB(r, g, b)
}

fn main() {
    let args = Args::from(std::env::args());
    let rom = args
        .positional(0)
        .unwrap_or_else(|| {
            println!("Usage: chip8 <rom> <background>");
            exit(1);
        })
        .clone();
    let background = to_color(args.option("background").unwrap_or("#000000".to_string()));
    let foreground = to_color(args.option("foreground").unwrap_or("#FFFFFF".to_string()));
    let volume = args
        .option("volume")
        .unwrap_or("0.25".to_string())
        .parse::<f32>()
        .unwrap_or_else(|_| {
            println!("Invalid volume value");
            exit(1);
        });
    let clock = args
        .option("clock")
        .unwrap_or("500".to_string())
        .parse::<u32>()
        .unwrap_or_else(|_| {
            println!("Invalid clock value");
            exit(1);
        });

    let cpu = Arc::new(Mutex::new(Chip8::new()));

    let t1_cpu = cpu.clone();

    let t1 = std::thread::spawn(move || {
        let rom = std::fs::read(rom).unwrap();
        t1_cpu.lock().unwrap().load_rom(rom.as_slice());

        loop {
            let mut cpu = t1_cpu.lock().unwrap();
            cpu.tick();
            if cpu.halted {
                break;
            }
            drop(cpu);
            std::thread::sleep(Duration::from_secs(1) / clock);
        }
    });

    let t2_cpu = cpu.clone();
    let t2 = std::thread::spawn(move || {
        // Inicializa SDL
        let sdl_context: Sdl = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();

        // Cria uma janela
        let window = video_subsystem
            .window("Emulador Chip-8", 640, 320)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let mut canvas = window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);

        // Loop de evento
        let mut event_pump = sdl_context.event_pump().unwrap();

        let desired_spec = sdl2::audio::AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };
        let device = audio_subsystem
            .open_playback(None, &desired_spec, |spec| {
                // initialize the audio callback
                SquareWave {
                    phase_inc: (440.0 * 2.0) / spec.freq as f32,
                    phase: 0.0,
                    volume,
                }
            })
            .unwrap();

        // Limpa a tela
        canvas.set_draw_color(background);
        canvas.clear();

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    sdl2::event::Event::Quit { .. } => {
                        cpu.lock().unwrap().halted = true;
                        break 'running;
                    }
                    sdl2::event::Event::KeyDown {
                        keycode: Some(key), ..
                    } => t2_cpu.lock().unwrap().on_key_down(get_key_mask(key)),
                    sdl2::event::Event::KeyUp {
                        keycode: Some(key), ..
                    } => t2_cpu.lock().unwrap().on_key_up(get_key_mask(key)),
                    // sdl2::event::Event::MouseButtonDown { window_id, .. }
                    // | sdl2::event::Event::MouseButtonUp { window_id, .. }
                    //     if window_id == keypad.window_id =>
                    // {
                    //     keypad.handle_event(event);
                    //     keypad.draw();
                    // }
                    _ => {}
                }
            }

            let buffer = t2_cpu.lock().unwrap().display;

            let (window_width, window_height) = canvas.output_size().unwrap();
            let scale_x = window_width / WIDTH;
            let scale_y = window_height / HEIGHT;
            let scale = scale_x.min(scale_y);

            // Desenha o buffer na tela
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let byte = buffer[(y * WIDTH + x) as usize / 8];
                    let bit = byte >> (7 - x % 8) & 1;

                    let color = if bit == 0 {
                        // Simula o efeito de fade dos monitores CRT
                        Color::RGBA(background.r, background.g, background.b, 48)
                    } else {
                        foreground
                    };

                    let rect = Rect::new(
                        x as i32 * scale as i32,
                        y as i32 * scale as i32,
                        scale,
                        scale,
                    );

                    canvas.set_draw_color(color);
                    canvas.fill_rect(rect).unwrap();
                }
            }

            std::thread::sleep(Duration::from_secs(1) / 60);

            // Atualiza o canvas
            canvas.present();

            // Update Audio
            if t2_cpu.lock().unwrap().sound_timer > 0
                && device.status() != sdl2::audio::AudioStatus::Playing
            {
                device.resume();
            } else if t2_cpu.lock().unwrap().sound_timer == 0
                && device.status() == sdl2::audio::AudioStatus::Playing
            {
                device.pause();
            }
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();
}

fn get_key_mask(key: sdl2::keyboard::Keycode) -> u16 {
    match key {
        sdl2::keyboard::Keycode::Num0 => KeyMask::Key0 as u16,
        sdl2::keyboard::Keycode::Num1 => KeyMask::Key1 as u16,
        sdl2::keyboard::Keycode::Num2 => KeyMask::Key2 as u16,
        sdl2::keyboard::Keycode::Num3 => KeyMask::Key3 as u16,
        sdl2::keyboard::Keycode::Num4 => KeyMask::Key4 as u16,
        sdl2::keyboard::Keycode::Num5 => KeyMask::Key5 as u16,
        sdl2::keyboard::Keycode::Num6 => KeyMask::Key6 as u16,
        sdl2::keyboard::Keycode::Num7 => KeyMask::Key7 as u16,
        sdl2::keyboard::Keycode::Num8 => KeyMask::Key8 as u16,
        sdl2::keyboard::Keycode::Num9 => KeyMask::Key9 as u16,
        sdl2::keyboard::Keycode::A => KeyMask::KeyA as u16,
        sdl2::keyboard::Keycode::B => KeyMask::KeyB as u16,
        sdl2::keyboard::Keycode::C => KeyMask::KeyC as u16,
        sdl2::keyboard::Keycode::D => KeyMask::KeyD as u16,
        sdl2::keyboard::Keycode::E => KeyMask::KeyE as u16,
        sdl2::keyboard::Keycode::F => KeyMask::KeyF as u16,
        _ => 0,
    }
}
