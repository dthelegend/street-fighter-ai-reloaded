use std::collections::HashMap;

use crate::retro::libretrocore::{LibRetroEnvironment, PixelFormat, FrameBuffer};


pub struct RetroEnvironmentManager {
    pub core_path: String,
    pub rom_path: String,
    environments: HashMap<String, LibRetroEnvironment>,
    id_ticker: usize
}

impl RetroEnvironmentManager {
    pub fn new(core_path: String, rom_path: String) -> RetroEnvironmentManager {
        RetroEnvironmentManager { id_ticker: 0, core_path, rom_path, environments: HashMap::new() }
    }

    pub fn get_frame_information_list(&self) -> Vec<(String, ProcessedFrameBuffer)> {
        self.environments.iter().map(|(k, v)| {
            (k.to_owned(), v.get_frame_information())
        })
            .filter_map(|(id, frame_info)| frame_info.map(|fi| (id, fi)))
            .map(|(id, x)| (id, ProcessedFrameBuffer::from(x)))
            .collect()
    }

    pub fn run_environments(&self) {
        for (_, env) in &self.environments {
            env.run()
        }
    }

    pub fn get_frame_information(&self, id: &String) -> Option<ProcessedFrameBuffer> {
        self.environments.get(id)
            .and_then(|env| env.get_frame_information())
            .map(ProcessedFrameBuffer::from)
    }

    pub fn create_environment(&mut self, name: Option<String>) -> Result<(), String> {
        let mut environment = LibRetroEnvironment::new(self.core_path.to_owned())?;

        environment.init();
        environment.load_rom(self.rom_path.to_owned())?;

        let final_name = if let Some(s) = name { s } else {
            loop {
                let env_name = format!("environment_{}", self.id_ticker);
                if !self.environments.contains_key(&env_name) { break env_name; };
                self.id_ticker += 1;
            }
        };

        self.environments.insert(final_name, environment);

        Ok(())
    }
}

pub struct ProcessedFrameBuffer {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pitch: usize
}

impl From<(PixelFormat, FrameBuffer)> for ProcessedFrameBuffer {
    fn from((format, fb): (PixelFormat, FrameBuffer)) -> Self {
        ProcessedFrameBuffer {
            buffer: match format {
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
        }
    }
}