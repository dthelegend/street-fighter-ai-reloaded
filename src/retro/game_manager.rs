use sdl2::{render::WindowCanvas, Sdl, EventPump};
use super::libretrocore::{LibRetroEnvironment, PixelFormat};

struct GameManager {
    environment: LibRetroEnvironment,
    gui_window_manager: Option<GuiWindowManager>
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

impl GameManager {
    fn new(core_path : String, rom_path : String, is_headless : bool) -> Result<GameManager, String> {
        let mut environment = LibRetroEnvironment::new(core_path)?;
        
        environment.init();
        environment.load_rom(rom_path)?;

        Ok(GameManager {
            gui_window_manager: if is_headless {
                let context = sdl2::init()?;
                let video_subsystem = context.video()?;

                let window = video_subsystem.window(DEFAULT_GUI_WINDOW_NAME, 800, 600)
                    .position_centered()
                    .build()
                    .expect("could not initialize video subsystem");

                let canvas = window.into_canvas()
                    .build()
                    .expect("could not make a canvas");
                
                let event_pump = context.event_pump()?;

                Some(GuiWindowManager {
                    context,
                    canvas,
                    event_pump
                })
            } else { None },
            environment
        })
    }

    fn render(&mut self) {
        let Some(mut wm) = self.gui_window_manager else { return };
        
        let frame_buffer = match self.environment.frame_buffer.lock() {
            Ok(fb_guard) => fb_guard.clone(),
            Err(_) => None
        };

        let format = match self.environment.frame_format.lock() {
            Ok(format_guard) => format_guard.clone(),
            Err(_) => PixelFormat::RetroPixelFormatUnknown
        };
        
        let processed_frame_buffer = frame_buffer.and_then(|fb| Some(ProcessedFrameBuffer{
            buffer: match format {
                PixelFormat::RetroPixelFormatUnknown => None,
                PixelFormat::RetroPixelFormatRgb1555 => Some(fb.buffer
                    .chunks(2)
                    .map(|chunk| ((chunk[0] as u16) << 8) | chunk[0] as u16)
                    .flat_map(|pixel_data|
                        [0,
                        ((pixel_data >> 1) & 0b11111) as u8,
                        ((pixel_data >> 6) & 0b11111) as u8,
                        ((pixel_data >> 11) & 0b11111) as u8])
                    .map(|color_pixel| color_pixel * (0b11111111 / 0b11111))
                    .collect()),
                PixelFormat::RetroPixelFormatRgb565 => Some(fb.buffer
                    .chunks(2)
                    .map(|chunk| ((chunk[0] as u16) << 8) | chunk[0] as u16)
                    .flat_map(|pixel_data|
                        [0,
                        (((pixel_data) & 0b11111) * (0b11111111 / 0b11111)) as u8,
                        (((pixel_data >> 5) & 0b111111) * (0b11111111 / 0b111111)) as u8,
                        (((pixel_data >> 11) & 0b11111) * (0b11111111 / 0b11111)) as u8])
                    .collect()),
                PixelFormat::RetroPixelFormatXrgb8888 => Some(fb.buffer)
            }?,
            height: fb.height,
            pitch: fb.pitch,
            width: fb.width
        }));
        
        if let Some(pfb) = processed_frame_buffer {
            let texture_creator = wm.canvas.texture_creator();
            let texture_result = texture_creator.create_texture(
                sdl2::pixels::PixelFormatEnum::RGB888,
                sdl2::render::TextureAccess::Static,
                pfb.width * pfb.pitch as u32,
                pfb.height);
            
            if let Ok(mut texture) = texture_result {
                texture.update(None,&pfb.buffer, pfb.pitch);
                wm.canvas.copy(&texture, None, None);
            }
        };

        wm.canvas.present();
    }
}