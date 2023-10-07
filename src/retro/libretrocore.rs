use std::{fs, ffi::{c_void, c_uint}, sync::Mutex};
use libffi::high::{Closure2, Closure0, Closure4};
use libretro_sys::{CoreAPI, GameInfo};
use libloading::Library;

const EXPECTED_LIB_RETRO_VERSION: u32 = 1;

pub struct LibRetroEnvironment {
    _dylib: Library,
    core_api: CoreAPI,
    pub core_path: String,
    pub rom_path: Option<String>,
    pub frame_buffer: Mutex<Option<FrameBuffer>>,
    pub frame_format: Mutex<Option<PixelFormat>>,
}

#[derive(Clone)]
pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pitch: usize
}

#[derive(Clone)]
pub enum PixelFormat {
    RetroPixelFormatRgb1555,
    RetroPixelFormatXrgb8888,
    RetroPixelFormatRgb565
}

impl LibRetroEnvironment {

    fn video_refresh_callback(&self, data: Option<Vec<u8>>, width: u32, height: u32, pitch: usize) {
        if let Ok(mut guard) = self.frame_buffer.lock() {
            *guard = data.map(|buffer| FrameBuffer { buffer, width, height, pitch });
        };
    }
    fn input_poll_callback(&self) {
        println!("libretro_set_input_poll_callback")
    }
    
    fn input_state_callback(&self, port: u32, device: u32, index: u32, id: u32) -> u16 {
        println!("libretro_set_input_state_callback");
        0// Hard coded 0 for now means nothing is pressed
    }
    
    fn audio_sample_callback(&self, left: i16, right: i16) {
        println!("libretro_set_audio_sample_callback");
    }
    
    fn audio_sample_batch_callback(&self, data: Option<Vec<i16>>, frames: usize) -> usize {
        println!("libretro_set_audio_sample_batch_callback");
        1
    }

    pub fn load_rom(&mut self, rom_path: String) -> Result<(), String> {
        unsafe {
            let rusty_data = fs::read(&rom_path).map_err(|err| format!("Failed to read reom into memory!\nReason: {}", err.kind()))?;
            
            let path = std::ffi::CString::new(rom_path.as_str()).map_err(|_| "Failed! String is Null")?.as_ptr();
            let data = rusty_data.as_ptr() as *const c_void;
            let meta = std::ptr::null();
            let size = rusty_data.len();

            let game_info = GameInfo { data, meta, path, size};
            
            if !((self.core_api.retro_load_game)(&game_info)) {
                return Err("Failed to load rom!".to_string());
            }
        }

        self.rom_path = Some(rom_path);

        Ok(())
    }

    pub fn init(&mut self) {
        // Environment Callback
        let env = |command: u32, return_data: *mut c_void| -> i32 {
            unsafe {
                match command {
                    libretro_sys::ENVIRONMENT_GET_CAN_DUPE => { *(return_data as *mut bool) = true; 0 },
                    libretro_sys::ENVIRONMENT_SET_PIXEL_FORMAT => {
                        const RGB565: u32 = libretro_sys::PixelFormat::RGB565 as u32;
                        const ARGB1555: u32 = libretro_sys::PixelFormat::ARGB1555 as u32;
                        const ARGB8888: u32 = libretro_sys::PixelFormat::ARGB8888 as u32;

                        let pixel_format = *(return_data as *const u32);
                        let processed_pixel_format = match pixel_format {
                            RGB565 => Some(PixelFormat::RetroPixelFormatRgb565),
                            ARGB1555 => Some(PixelFormat::RetroPixelFormatRgb1555),
                            ARGB8888 => Some(PixelFormat::RetroPixelFormatXrgb8888),
                            _ => None
                        };
                        println!("Set ENVIRONMENT_SET_PIXEL_FORMAT to: {}", pixel_format);

                        if let Ok(mut guard) = self.frame_format.lock() {
                            *guard = processed_pixel_format
                        }

                        1
                    }
                    _ => {
                        println!("libretro_environment_callback Called with command: {}", command);
                        
                        0
                    }
                }
            }
        };
        let env_closure = Closure2::new(&env);
        let &env_code = env_closure.code_ptr();
        let env_ptr:unsafe extern "C" fn(u32, *mut std::ffi::c_void) -> bool = unsafe { std::mem::transmute(env_code) };
        
        // Set Video Refresh Callback
        let svr = |data : *const std::ffi::c_void, width: c_uint, height : c_uint, pitch: usize| unsafe {
            let processed_data =
                if data.is_null() { None }
                else { Some(Vec::from(std::slice::from_raw_parts(data as *const u8, (width * height) as usize))) };

            self.video_refresh_callback(processed_data, width, height, pitch);
        };
        let svr_closure = Closure4::new(&svr);
        let &svr_code = svr_closure.code_ptr();
        let svr_ptr:unsafe extern "C" fn(*const std::ffi::c_void, u32, u32, usize) = unsafe { std::mem::transmute(svr_code) };

        // Set Input Poll Callback
        let sip = || {
            self.input_poll_callback()
        };
        let sip_closure = Closure0::new(&sip);
        let &sip_code = sip_closure.code_ptr();
        let sip_ptr:unsafe extern "C" fn() = unsafe { std::mem::transmute(sip_code) };

        // Set Input State Callback
        let sis = |port: c_uint, device: c_uint, index: c_uint, id: c_uint| -> u16 {
            self.input_state_callback(port, device, index, id)
        };
        let sis_closure = Closure4::new(&sis);
        let &sis_code = sis_closure.code_ptr();
        let sis_ptr:unsafe extern "C" fn(u32, _, _, u32) -> i16 = unsafe { std::mem::transmute(sis_code) };

        // Set Audio Sample Callback
        let sas = |left: i16, right: i16| {
            self.audio_sample_callback(left, right)
        };
        let sas_closure = Closure2::new(& sas);
        let &sas_code = sas_closure.code_ptr();
        let sas_ptr:unsafe extern "C" fn(i16, i16) = unsafe { std::mem::transmute(sas_code) };

        // Set Audio Sample Batch Callback
        let  sasb = |data: *const i16, frames: usize| -> usize { unsafe {
            let processed_data =
                if data.is_null() { None }
                else {Some(Vec::from(std::slice::from_raw_parts(data, frames))) };
        
            self.audio_sample_batch_callback(processed_data, frames)
        }};
        let sasb_closure = Closure2::new(& sasb);
        let &sasb_code = sasb_closure.code_ptr();
        let sasb_ptr:unsafe extern "C" fn(*const i16, usize) -> usize = unsafe { std::mem::transmute(sasb_code) };

        // TODO: Remove and replace as to not leak memory!!!!
        std::mem::forget(env_closure);
        std::mem::forget(svr_closure);
        std::mem::forget(sip_closure);
        std::mem::forget(sis_closure);
        std::mem::forget(sas_closure);
        std::mem::forget(sasb_closure);

        unsafe {
            (self.core_api.retro_set_environment)(env_ptr);
            (self.core_api.retro_init)();
            (self.core_api.retro_set_video_refresh)(svr_ptr);
            (self.core_api.retro_set_input_poll)(sip_ptr);
            (self.core_api.retro_set_input_state)(sis_ptr);
            (self.core_api.retro_set_audio_sample)(sas_ptr);
            (self.core_api.retro_set_audio_sample_batch)(sasb_ptr);
        }
    }

    pub fn run(&self) {
        unsafe {
            (self.core_api.retro_run)();
        }
    }

    pub fn get_frame_information(&self) -> Option<(PixelFormat, FrameBuffer)> {
        let frame_format = match self.frame_format.lock() {
            Ok(format_guard) => format_guard.clone(),
            Err(_) => None
        };

        let frame_buffer = match self.frame_buffer.lock() {
            Ok(fb_guard) => fb_guard.clone(),
            Err(_) => None
        };

        match (frame_format, frame_buffer) {
            (Some(format), Some(buf)) => Some((format, buf)),
            (_, _ ) => None
        }
    }

    pub fn new(core_path: String) -> Result<LibRetroEnvironment, String>{
        unsafe {
            let dylib = Library::new(&core_path).map_err(|_| "Failed to load Core")?;
        
            let core_api = CoreAPI {
                retro_set_environment: *(dylib.get(b"retro_set_environment").map_err(|_| "Failed to load Core")?),
                retro_set_video_refresh: *(dylib.get(b"retro_set_video_refresh").map_err(|_| "Failed to load Core")?),
                retro_set_audio_sample: *(dylib.get(b"retro_set_audio_sample").map_err(|_| "Failed to load Core")?),
                retro_set_audio_sample_batch: *(dylib.get(b"retro_set_audio_sample_batch").map_err(|_| "Failed to load Core")?),
                retro_set_input_poll: *(dylib.get(b"retro_set_input_poll").map_err(|_| "Failed to load Core")?),
                retro_set_input_state: *(dylib.get(b"retro_set_input_state").map_err(|_| "Failed to load Core")?),

                retro_init: *(dylib.get(b"retro_init").map_err(|_| "Failed to load Core")?),
                retro_deinit: *(dylib.get(b"retro_deinit").map_err(|_| "Failed to load Core")?),

                retro_api_version: *(dylib.get(b"retro_api_version").map_err(|_| "Failed to load Core")?),

                retro_get_system_info: *(dylib.get(b"retro_get_system_info").map_err(|_| "Failed to load Core")?),
                retro_get_system_av_info: *(dylib.get(b"retro_get_system_av_info").map_err(|_| "Failed to load Core")?),
                retro_set_controller_port_device: *(dylib.get(b"retro_set_controller_port_device").map_err(|_| "Failed to load Core")?),

                retro_reset: *(dylib.get(b"retro_reset").map_err(|_| "Failed to load Core")?),
                retro_run: *(dylib.get(b"retro_run").map_err(|_| "Failed to load Core")?),

                retro_serialize_size: *(dylib.get(b"retro_serialize_size").map_err(|_| "Failed to load Core")?),
                retro_serialize: *(dylib.get(b"retro_serialize").map_err(|_| "Failed to load Core")?),
                retro_unserialize: *(dylib.get(b"retro_unserialize").map_err(|_| "Failed to load Core")?),

                retro_cheat_reset: *(dylib.get(b"retro_cheat_reset").map_err(|_| "Failed to load Core")?),
                retro_cheat_set: *(dylib.get(b"retro_cheat_set").map_err(|_| "Failed to load Core")?),

                retro_load_game: *(dylib.get(b"retro_load_game").map_err(|_| "Failed to load Core")?),
                retro_load_game_special: *(dylib.get(b"retro_load_game_special").map_err(|_| "Failed to load Core")?),
                retro_unload_game: *(dylib.get(b"retro_unload_game").map_err(|_| "Failed to load Core")?),

                retro_get_region: *(dylib.get(b"retro_get_region").map_err(|_| "Failed to load Core")?),
                retro_get_memory_data: *(dylib.get(b"retro_get_memory_data").map_err(|_| "Failed to load Core")?),
                retro_get_memory_size: *(dylib.get(b"retro_get_memory_size").map_err(|_| "Failed to load Core")?),
            };
        
            let api_version = (core_api.retro_api_version)();
        
            if api_version != EXPECTED_LIB_RETRO_VERSION {
                return Err(format!("This core has been compiled with an incorrect LibRetro API version.\nGot: {}\nExpected: {}", api_version, EXPECTED_LIB_RETRO_VERSION));
            }

            let lib_retro_environment = LibRetroEnvironment {
                core_path: core_path.to_owned(),
                _dylib: dylib,
                core_api,
                frame_format: Mutex::new(None),
                frame_buffer: Mutex::new(None),
                rom_path: None
            };

            Ok(lib_retro_environment)
        }
    }
}
