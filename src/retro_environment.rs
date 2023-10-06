use std::collections::HashMap;

use sdl2::{render::WindowCanvas, Sdl, EventPump};
use super::retro::libretrocore::{LibRetroEnvironment, PixelFormat};
use pyo3::prelude::*;

struct GameEnvironment<'a> {
    id: usize,
    parent: &'a GameEnvironmentBuilder,
    name: String,
    environment: &'a LibRetroEnvironment
}

#[pyclass]
struct GameEnvironmentBuilder {
    pub core_path: String,
    pub rom_path: String,
    environments: HashMap<usize, LibRetroEnvironment>,
    id_ticker: usize
}

impl GameEnvironmentBuilder {
    pub fn new(core_path: String, rom_path: String) -> GameEnvironmentBuilder {
        return GameEnvironmentBuilder { id_ticker: 0, core_path, rom_path, environments: HashMap::new() };
    }

    pub fn create_environment(&mut self, name: Option<String>) -> Result<GameEnvironment, String> {
        let mut environment = LibRetroEnvironment::new(self.core_path)?;

        environment.init();
        environment.load_rom(self.rom_path)?;

        self.environments.insert(self.id_ticker, environment);

        let final_env = GameEnvironment {
            id: self.id_ticker,
            parent: self,
            name: name.unwrap_or_else(|| "Unbeleivable".to_owned()), // TODO: Replace with cool random names
            environment: &environment
        };

        self.id_ticker += 1;

        Ok(final_env)
    }
}

impl Drop for GameEnvironment<'_> {
    fn drop(&mut self) {
        self.parent.environments.remove(&self.id);
    }
}

struct GuiWindowManager {
    context: Sdl,
    canvas: WindowCanvas,
    event_pump: EventPump
}

const DEFAULT_GUI_WINDOW_NAME : &str = "Retro";

struct ProcessedFrameBuffer {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pitch: usize
}

impl GuiWindowManager {
    fn new() -> Result<GuiWindowManager, String> {
        let context = sdl2::init()?;
        let video_subsystem = context.video()?;

        let window = video_subsystem.window(DEFAULT_GUI_WINDOW_NAME, 800, 600)
            .position_centered()
            .build()
            .map_err(|_| "could not initialize video subsystem".to_owned())?;

        let canvas = window.into_canvas()
            .build()
            .map_err(|_| "could not make a canvas".to_owned())?;
        
        let event_pump = context.event_pump()?;

        Ok(GuiWindowManager {
            context,
            canvas,
            event_pump
        })
    }

    fn render(&mut self, environments: Vec<LibRetroEnvironment>) {
        let frame_buffers = environments.iter().map(|env|
            (match env.frame_format.lock() {
                Ok(format_guard) => format_guard.clone(),
                Err(_) => PixelFormat::RetroPixelFormatUnknown
            },
            match env.frame_buffer.lock() {
                Ok(fb_guard) => fb_guard.clone(),
                Err(_) => None
            }))
            .filter(|(form, fb)| fb.is_some() && !matches!(form, PixelFormat::RetroPixelFormatUnknown))
            .map(|(form, fb)| (form, fb.unwrap()));

        let processed_frame_buffers = frame_buffers.map(|(format, fb)|
            ProcessedFrameBuffer {
                buffer: match format {
                    PixelFormat::RetroPixelFormatUnknown => panic!(),
                    PixelFormat::RetroPixelFormatRgb1555 => fb.buffer
                        .chunks(2)
                        .map(|chunk| ((chunk[0] as u16) << 8) | chunk[0] as u16)
                        .flat_map(|pixel_data|
                            [0,
                            ((pixel_data >> 1) & 0b11111) as u8,
                            ((pixel_data >> 6) & 0b11111) as u8,
                            ((pixel_data >> 11) & 0b11111) as u8])
                        .map(|color_pixel| color_pixel * (0b11111111 / 0b11111))
                        .collect(),
                    PixelFormat::RetroPixelFormatRgb565 => fb.buffer
                        .chunks(2)
                        .map(|chunk| ((chunk[0] as u16) << 8) | chunk[0] as u16)
                        .flat_map(|pixel_data|
                            [0,
                            (((pixel_data) & 0b11111) * (0b11111111 / 0b11111)) as u8,
                            (((pixel_data >> 5) & 0b111111) * (0b11111111 / 0b111111)) as u8,
                            (((pixel_data >> 11) & 0b11111) * (0b11111111 / 0b11111)) as u8])
                        .collect(),
                    PixelFormat::RetroPixelFormatXrgb8888 => fb.buffer
                },
                height: fb.height,
                pitch: fb.pitch,
                width: fb.width
            });
        
        let textures = processed_frame_buffers.map(|pfb| {
            let texture_creator = self.canvas.texture_creator();
            let texture_result = texture_creator.create_texture(
                sdl2::pixels::PixelFormatEnum::RGB888,
                sdl2::render::TextureAccess::Static,
                pfb.width * pfb.pitch as u32,
                pfb.height);
            
            if let Ok(mut texture) = texture_result {
                texture.update(None,&pfb.buffer, pfb.pitch);
            }
            
            texture_result
        });
        
        for texture in textures {
            if let Ok(mut texture) = texture {
                self.canvas.copy(&texture, None, None);
            }
        }

        self.canvas.present();
    }
}