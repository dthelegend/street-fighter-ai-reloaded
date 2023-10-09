use std::{ffi::{c_void, c_uint, CStr}, path::PathBuf, fs::canonicalize};
use libffi::high::{ClosureMut2, Closure0, ClosureMut4};
use crate::retro::libretro_sys;

const EXPECTED_LIB_RETRO_VERSION: u32 = 1;

pub struct LibRetroEnvironment {
    core_api: libretro_sys::LibretroAPI,
    pub core_path: String,
    pub rom_path: Option<String>,
    frame_buffer: Option<FrameBuffer>,
    frame_format: Option<PixelFormat>
}

#[derive(Clone)]
pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pitch: usize
}

#[derive(Clone, Debug)]
pub enum PixelFormat {
    RetroPixelFormatRgb1555,
    RetroPixelFormatXrgb8888,
    RetroPixelFormatRgb565
}

unsafe extern "C" fn logger(level: libretro_sys::retro_log_level, fmt: *const std::ffi::c_char, mut stuff: ...) {
    let error_level = match level {
        libretro_sys::retro_log_level_RETRO_LOG_DUMMY => "DUMMY",
        libretro_sys::retro_log_level_RETRO_LOG_DEBUG => "DEBUG",
        libretro_sys::retro_log_level_RETRO_LOG_INFO => "INFO",
        libretro_sys::retro_log_level_RETRO_LOG_WARN => "WARN",
        libretro_sys::retro_log_level_RETRO_LOG_ERROR => "ERROR",
        _ => "UNKNOWN"
    };

    let fmtd_str: Option<String> = unsafe {
        const SIZE_OF_CHAR : usize = std::mem::size_of::<std::ffi::c_char>();
        const MAX_MESSAGE_SIZE : usize = 200 * SIZE_OF_CHAR;

        let ptr: *mut std::ffi::c_char = libc::malloc(MAX_MESSAGE_SIZE).cast();

        let n = libc::snprintf(ptr, MAX_MESSAGE_SIZE, fmt, ..stuff.as_va_list());
        
        if n <= 0 {
            libc::free(ptr.cast());
            
            None
        }
        else {
            let new_size = std::cmp::max((n + 1) as usize, MAX_MESSAGE_SIZE);
            libc::realloc(ptr.cast(), new_size);
            *((ptr as usize + (new_size - SIZE_OF_CHAR)) as *mut std::ffi::c_char) = 0;

            let a = match CStr::from_ptr(ptr).to_str() {
                Ok(b) => Some(b.to_owned()),
                Err(e) => {
                    println!("Failed to log: (Size: {:?}) {:?}", new_size, e);

                    None
                }
            };

            libc::free(ptr.cast());

            a
        }
    };

    if let Some(valid_fmtd_str) = fmtd_str {
        print!("[{}] {}", error_level, valid_fmtd_str);
    }
}

const LOGGER_CALLBACK : libretro_sys::retro_log_callback = libretro_sys::retro_log_callback {
    log: Some(logger)
};

impl LibRetroEnvironment {
    
    pub fn load_rom(&mut self, rom_path_string: String) -> Result<(), String> {
        let rom_path = PathBuf::from(rom_path_string);
        let resolved_rom_path = canonicalize(rom_path)
            .map_err(|_| "Unable to canonise path".to_owned())?;
        let resolved_path_str = resolved_rom_path.to_str().ok_or_else(|| "Unable to convert resolved path to string ".to_owned())?;

        println!("Attempting to load rom at {:?}", resolved_path_str);
        // let rusty_data = fs::read(&rom_path).map_err(|err| format!("Failed to read ROM into memory!\nReason: {}", err.kind()))?;
        
        let path = std::ffi::CString::new(resolved_path_str).map_err(|_| "Failed! Path string is Null")?.as_ptr();
        // let data = rusty_data.as_ptr() as *const c_void;
        let data = std::ptr::null();
        let meta = std::ptr::null();
        // let size = rusty_data.len();
        let size = 0;

        let game_info = libretro_sys::retro_game_info { data, meta, path, size};
        
        unsafe {
            if !(self.core_api.retro_load_game(&game_info)) {
                return Err("Failed to load rom!".to_string());
            }
        }
        
        Ok(())
    }

    pub fn init(&mut self) {
        // Environment Callback
        let mut env = |command: u32, return_data: *mut c_void| -> usize {
            match command {
                libretro_sys::RETRO_ENVIRONMENT_GET_LOG_INTERFACE => unsafe { 
                    println!("Hooking logger into Envrironment");
                
                    *(return_data as *mut libretro_sys::retro_log_callback) = LOGGER_CALLBACK;

                    1
                },
                libretro_sys::RETRO_ENVIRONMENT_GET_CAN_DUPE =>  {
                    unsafe { *(return_data as *mut bool) = true; }
                    1
                },
                libretro_sys::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
                    let pixel_format = unsafe { *(return_data as *const u32) };
                    let processed_pixel_format = match pixel_format {
                        libretro_sys::retro_pixel_format_RETRO_PIXEL_FORMAT_RGB565 => Some(PixelFormat::RetroPixelFormatRgb565),
                        libretro_sys::retro_pixel_format_RETRO_PIXEL_FORMAT_0RGB1555 => Some(PixelFormat::RetroPixelFormatRgb1555),
                        libretro_sys::retro_pixel_format_RETRO_PIXEL_FORMAT_XRGB8888 => Some(PixelFormat::RetroPixelFormatXrgb8888),
                        _ => None
                    };

                    println!("LibRetro requested PixelFormat {:?}", if let Some(format) = &processed_pixel_format { format!("{:?}", format) } else { "none".to_owned() });

                    self.frame_format = processed_pixel_format;

                    1
                }
                libretro_sys::RETRO_ENVIRONMENT_GET_GAME_INFO_EXT => 0,
                _ => {
                    println!("libretro_environment_callback Called with command: {}", command);
                    
                    0
                }
            }
        };
        let env_closure = ClosureMut2::new(&mut env);
        let &env_code = env_closure.code_ptr();
        let env_ptr:unsafe extern "C" fn(u32, *mut std::ffi::c_void) -> bool = unsafe { std::mem::transmute(env_code) };
        
        // Set Video Refresh Callback
        let mut svr = |data : *const std::ffi::c_void, width: c_uint, height : c_uint, pitch: usize| unsafe {
            println!("libretro_set_video_refresh_callback");

            let processed_data =
                if data.is_null() { None }
                else { Some(Vec::from(std::slice::from_raw_parts(data as *const u8, (width * height) as usize))) };

            self.frame_buffer = processed_data.map(|buffer| FrameBuffer { buffer, width, height, pitch });  
        };
        let svr_closure = ClosureMut4::new(&mut svr);
        let &svr_code = svr_closure.code_ptr();
        let svr_ptr:unsafe extern "C" fn(*const std::ffi::c_void, u32, u32, usize) = unsafe { std::mem::transmute(svr_code) };

        // Set Input Poll Callback
        let mut sip = || {
            println!("libretro_set_input_poll_callback")
        };
        let sip_closure = Closure0::new(&sip);
        let &sip_code = sip_closure.code_ptr();
        let sip_ptr:unsafe extern "C" fn() = unsafe { std::mem::transmute(sip_code) };

        // Set Input State Callback
        let mut sis = move |port: c_uint, device: c_uint, index: c_uint, id: c_uint| -> u16 {
            println!("libretro_set_input_state_callback");
            0// Hard coded 0 for now means nothing is pressed
        };
        let sis_closure = ClosureMut4::new(&mut sis);
        let &sis_code = sis_closure.code_ptr();
        let sis_ptr:unsafe extern "C" fn(u32, _, _, u32) -> i16 = unsafe { std::mem::transmute(sis_code) };

        // Set Audio Sample Callback
        let mut sas = move |left: i16, right: i16| {
            println!("libretro_set_audio_sample_batch_callback");
            1
        };
        let sas_closure = ClosureMut2::new(&mut sas);
        let &sas_code = sas_closure.code_ptr();
        let sas_ptr:unsafe extern "C" fn(i16, i16) = unsafe { std::mem::transmute(sas_code) };

        // Set Audio Sample Batch Callback
        let mut sasb = move |data: *const i16, frames: usize| -> usize { unsafe {
            let processed_data =
                if data.is_null() { None }
                else {Some(Vec::from(std::slice::from_raw_parts(data, frames))) };

            println!("libretro_set_audio_sample_callback");
            1
        }};
        let sasb_closure = ClosureMut2::new(&mut sasb);
        let &sasb_code = sasb_closure.code_ptr();
        let sasb_ptr:unsafe extern "C" fn(*const i16, usize) -> usize = unsafe { std::mem::transmute(sasb_code) };

        // TODO: Remove and replace as to not leak memory!!!!
        std::mem::forget(env_closure);
        std::mem::forget(svr_closure);
        std::mem::forget(sip_closure);
        std::mem::forget(sis_closure);
        std::mem::forget(sas_closure);
        std::mem::forget(sasb_closure);

        println!("Setting Retro Environment callback");
        unsafe {
            self.core_api.retro_set_environment(Some(env_ptr));
        }
        println!("Initialising Retro Environment");
        unsafe {
            self.core_api.retro_init();
        }
        println!("Setting remaining Retro Callbacks");
        unsafe {
            self.core_api.retro_set_video_refresh(Some(svr_ptr));
            self.core_api.retro_set_input_poll(Some(sip_ptr));
            self.core_api.retro_set_input_state(Some(sis_ptr));
            self.core_api.retro_set_audio_sample(Some(sas_ptr));
            self.core_api.retro_set_audio_sample_batch(Some(sasb_ptr));
        }
    }

    pub fn run(&self) {
        unsafe {
            self.core_api.retro_run();
        }
    }

    pub fn get_frame_information(&self) -> Option<(PixelFormat, FrameBuffer)> {

        match (self.frame_format.clone(), self.frame_buffer.clone()) {
            (Some(format), Some(buf)) => Some((format, buf)),
            (_, _ ) => None
        }
    }

    pub fn new(core_path: String) -> Result<LibRetroEnvironment, String>{
        unsafe {
            let core_api = libretro_sys::LibretroAPI::new(&core_path)
                .map_err(|_| format!("Failed to create core from \"{}\"", core_path))?;
        
            let api_version = core_api.retro_api_version();
        
            if api_version != libretro_sys::RETRO_API_VERSION {
                return Err(format!("This core has been compiled with an incompatible LibRetro API version.\nGot: {}\nExpected: {}", api_version, EXPECTED_LIB_RETRO_VERSION));
            }

            print!("Successfully loaded core from {:?}", core_path);

            let lib_retro_environment = LibRetroEnvironment {
                core_path: core_path.to_owned(),
                core_api,
                frame_format: None,
                frame_buffer: None,
                rom_path: None
            };

            Ok(lib_retro_environment)
        }
    }
}
