use std::{ffi::{c_void, c_uint}, sync::{Mutex, Arc}, rc::Rc};
use crate::retro::libretro_sys;

const EXPECTED_LIB_RETRO_VERSION: u32 = 1;

static mut GLOBAL_LIBRETRO_ENVIRONMENT: LibretroEnvrironmentState = LibretroEnvrironmentState::NoCoreLoaded;

#[derive(Clone, Debug)]
pub struct FrameBuffer {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pitch: usize
}

#[derive(Clone, Copy, Debug)]
pub enum PixelFormat {
    RetroPixelFormatRgb1555,
    RetroPixelFormatXrgb8888,
    RetroPixelFormatRgb565,
    RetroPixelFormatUnknown
}

impl From<u32> for PixelFormat {
    fn from(value: u32) -> Self {
        match value {
            libretro_sys::retro_pixel_format_RETRO_PIXEL_FORMAT_RGB565 => PixelFormat::RetroPixelFormatRgb565,
            libretro_sys::retro_pixel_format_RETRO_PIXEL_FORMAT_0RGB1555 => PixelFormat::RetroPixelFormatRgb1555,
            libretro_sys::retro_pixel_format_RETRO_PIXEL_FORMAT_XRGB8888 => PixelFormat::RetroPixelFormatXrgb8888,
            _ => PixelFormat::RetroPixelFormatUnknown
        }
    }
}

#[derive(Clone)]
enum LibretroEnvrironmentState {
    NoCoreLoaded,
    CoreLoaded(LoadedCore),
    CoreInitialised(InitialisedCore),
    CoreInitialisedWithRom(InitialisedCoreWithRom)
}

pub struct GlobalLibretroEnvironmentManager;

impl GlobalLibretroEnvironmentManager {
    pub fn load_core(self, core_path: String) -> Result<Self, String> {
        if unsafe {!matches!(GLOBAL_LIBRETRO_ENVIRONMENT, LibretroEnvrironmentState::NoCoreLoaded)} {
            Err("Core already loaded".to_owned())
        }
        else {
            let new_core = LibretroEnvrironmentState::CoreLoaded(LoadedCore::new(core_path)?);

            unsafe { GLOBAL_LIBRETRO_ENVIRONMENT = new_core };

            Ok(self)
        }
    }

    pub fn initialise_core(self) -> Result<Self, String> {
        unsafe { 
            let initialised_core = match GLOBAL_LIBRETRO_ENVIRONMENT.clone() {
                LibretroEnvrironmentState::NoCoreLoaded => Err("No core currently loaded!".to_owned()),
                LibretroEnvrironmentState::CoreLoaded(core) => Ok(core),
                LibretroEnvrironmentState::CoreInitialised(_) => Err("Core already initialised!".to_owned()),
                LibretroEnvrironmentState::CoreInitialisedWithRom(_) => Err("Core already has ROM loaded!".to_owned())
            }?.initialise()?;

            GLOBAL_LIBRETRO_ENVIRONMENT = LibretroEnvrironmentState::CoreInitialised(initialised_core);
        }

        Ok(self)
    }

    pub fn load_rom(self, rom_path: String) -> Result<Self, String> {
        unsafe { 
            let initialised_core_with_rom = match GLOBAL_LIBRETRO_ENVIRONMENT.clone() {
                LibretroEnvrironmentState::NoCoreLoaded => Err("No core currently loaded!".to_owned()),
                LibretroEnvrironmentState::CoreLoaded(_) => Err("Core must be initialised before loading ROM!".to_owned()),
                LibretroEnvrironmentState::CoreInitialised(core) => core.load_rom(rom_path),
                LibretroEnvrironmentState::CoreInitialisedWithRom(_) => Err("Core already has ROM loaded!".to_owned())
            }?;

            GLOBAL_LIBRETRO_ENVIRONMENT = LibretroEnvrironmentState::CoreInitialisedWithRom(initialised_core_with_rom);
        };

        Ok(self)
    }

    pub fn unload_rom(self) -> Result<Self, String> {
        unsafe {
            let core_initialised = match GLOBAL_LIBRETRO_ENVIRONMENT.clone() {
                LibretroEnvrironmentState::NoCoreLoaded => Err("No core currently loaded!".to_owned()),
                LibretroEnvrironmentState::CoreLoaded(_) => Err("Core must be initialised before unloading ROM!".to_owned()),
                LibretroEnvrironmentState::CoreInitialised(_) => Err("Core already has no ROM loaded!".to_owned()),
                LibretroEnvrironmentState::CoreInitialisedWithRom(core) => Ok(core.unload_rom())
            }?;

            GLOBAL_LIBRETRO_ENVIRONMENT = LibretroEnvrironmentState::CoreInitialised(core_initialised);
        }

        Ok(self)
    }

    pub fn run(self) -> Result<Self, String> {
        unsafe {
            match GLOBAL_LIBRETRO_ENVIRONMENT.clone() {
                LibretroEnvrironmentState::NoCoreLoaded => Err("No core currently loaded!".to_owned()),
                LibretroEnvrironmentState::CoreLoaded(_) => Err("Core must be initialised before running!".to_owned()),
                LibretroEnvrironmentState::CoreInitialised(_) => Err("Core has no ROM loaded!".to_owned()),
                LibretroEnvrironmentState::CoreInitialisedWithRom(core) => Ok(core.run())
            }?;
        }

        Ok(self)
    }

    pub fn get_frame_info(&self) -> Result<FrameBuffer, String> {
        match unsafe { GLOBAL_LIBRETRO_ENVIRONMENT.clone() } {
            LibretroEnvrironmentState::NoCoreLoaded => Err("No core currently loaded!".to_owned()),
            LibretroEnvrironmentState::CoreLoaded(_) => Err("Core must be initialised before getting frame info!".to_owned()),
            LibretroEnvrironmentState::CoreInitialised(_) => Err("Core has no ROM loaded!".to_owned()),
            LibretroEnvrironmentState::CoreInitialisedWithRom(core) => core.get_frame_info()
        }
    }
}

#[derive(Clone)]
struct LoadedCore {
    core_api: Arc<libretro_sys::LibretroAPI>,
    core_path: String
}

unsafe extern "C" fn logger(level: libretro_sys::retro_log_level, fmt: *const std::ffi::c_char, mut var_args: ...) {
    let error_level = match level {
        libretro_sys::retro_log_level_RETRO_LOG_DUMMY => "DUMMY",
        libretro_sys::retro_log_level_RETRO_LOG_DEBUG => "DEBUG",
        libretro_sys::retro_log_level_RETRO_LOG_INFO => "INFO",
        libretro_sys::retro_log_level_RETRO_LOG_WARN => "WARN",
        libretro_sys::retro_log_level_RETRO_LOG_ERROR => "ERROR",
        _ => "UNKNOWN"
    };

    // Guess?!?! The number of args in the var args by counting the 

    let mut fmtd_str = String::new();

    printf_compat::format(fmt, var_args.as_va_list(), printf_compat::output::fmt_write(&mut fmtd_str));

    print!("[{}] {}", error_level, fmtd_str);
}

const LOGGER_CALLBACK : libretro_sys::retro_log_callback = libretro_sys::retro_log_callback {
    log: Some(logger)
};

unsafe extern "C" fn on_video_refresh(data : *const c_void, width: c_uint, height : c_uint, pitch: usize) {
    println!("libretro_set_video_refresh_callback");

    unsafe {
        if let LibretroEnvrironmentState::CoreInitialisedWithRom(mut core) = GLOBAL_LIBRETRO_ENVIRONMENT.clone() {
            core.frame_buffer =
                if data.is_null() { None }
                else { Some(unsafe { std::slice::from_raw_parts(data as *const u8, (width * height) as usize) }) }
                .map(Vec::from)
                .map(|buffer| FrameBuffer { buffer, width, height, pitch });

            GLOBAL_LIBRETRO_ENVIRONMENT = LibretroEnvrironmentState::CoreInitialisedWithRom(core)
        };
    }
}

unsafe extern "C" fn on_set_environment(command: c_uint, return_data: *mut c_void) -> bool {
    match command {
        libretro_sys::RETRO_ENVIRONMENT_GET_LOG_INTERFACE => unsafe { 
            println!("Hooking logger into Envrironment");
        
            *(return_data as *mut libretro_sys::retro_log_callback) = LOGGER_CALLBACK;

            true
        },
        libretro_sys::RETRO_ENVIRONMENT_GET_CAN_DUPE =>  {
            unsafe { *(return_data as *mut bool) = true; }
            
            true
        },
        libretro_sys::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
            let pixel_format = PixelFormat::from(unsafe { *(return_data as *const u32) });

            println!("LibRetro requested PixelFormat {:?}", pixel_format);

            GLOBAL_LIBRETRO_ENVIRONMENT = match GLOBAL_LIBRETRO_ENVIRONMENT.clone() {
                LibretroEnvrironmentState::CoreInitialised(mut core) => {
                    core.frame_format = pixel_format;
                
                    LibretroEnvrironmentState::CoreInitialised(core)
                },
                LibretroEnvrironmentState::CoreInitialisedWithRom(mut core) => {
                    core.frame_format = pixel_format;
                
                    LibretroEnvrironmentState::CoreInitialisedWithRom(core)
                },
                some_core => some_core
            };

            true
        }
        libretro_sys::RETRO_ENVIRONMENT_GET_GAME_INFO_EXT => {
            println!("RETRO_ENVIRONMENT_GET_GAME_INFO_EXT ddeclined");
            false
        },
        _ => {
            println!("libretro_environment_callback Called with command: {}", command);
            
            false
        }
    }
}

unsafe extern "C" fn on_input_poll() {
    println!("libretro_set_input_poll_callback")
}

unsafe extern "C" fn on_input_state(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16 {
    println!("libretro_set_input_state_callback");
    0// Hard coded 0 for now means nothing is pressed
}

unsafe extern "C" fn on_audio_sample(left: i16, right: i16) {
    println!("libretro_set_audio_sample_batch_callback");
}

unsafe extern "C" fn on_audio_sample_batch(data: *const i16, frames: usize) -> usize {
    let processed_data =
        if data.is_null() { None }
        else {Some(Vec::from(std::slice::from_raw_parts(data, frames))) };

    println!("libretro_set_audio_sample_callback");
    1
}

impl LoadedCore {
    fn new(core_path: String) -> Result<LoadedCore, String> {
        unsafe {
            println!("Attempting to load core from {:?}", core_path);
            let core_api = Arc::new(libretro_sys::LibretroAPI::new(&core_path)
                .map_err(|_| format!("Failed to create core from \"{}\"", core_path))?);
        
            let api_version = core_api.retro_api_version();
        
            if api_version != libretro_sys::RETRO_API_VERSION {
                return Err(format!("This core has been compiled with an incompatible LibRetro API version.\nGot: {}\nExpected: {}", api_version, EXPECTED_LIB_RETRO_VERSION));
            }

            println!("Successfully loaded core");

            Ok(LoadedCore { core_api, core_path })
        }
    }

    fn initialise(self) -> Result<InitialisedCore, String> {
        let Self { core_api, core_path } = self;

        println!("Setting Retro Environment callback");
        unsafe {
            core_api.retro_set_environment(Some(on_set_environment));
        }
        println!("Initialising Retro Environment");
        unsafe {
            core_api.retro_init();
        }
        println!("Setting remaining Retro Callbacks");
        unsafe {
            core_api.retro_set_video_refresh(Some(on_video_refresh));
            core_api.retro_set_input_poll(Some(on_input_poll));
            core_api.retro_set_input_state(Some(on_input_state));
            core_api.retro_set_audio_sample(Some(on_audio_sample));
            core_api.retro_set_audio_sample_batch(Some(on_audio_sample_batch));
        }
        
        Ok(InitialisedCore {
            core_api,
            core_path,
            frame_format: PixelFormat::RetroPixelFormatUnknown
        })
    }
}

#[derive(Clone)]
struct InitialisedCore {
    core_api: Arc<libretro_sys::LibretroAPI>,
    core_path: String,
    frame_format: PixelFormat
}
    
impl InitialisedCore  {
    fn load_rom(self, rom_path: String) -> Result<InitialisedCoreWithRom, String> {
        let InitialisedCore { core_api, core_path, frame_format } = self;

        let path = std::ffi::CString::new(rom_path.clone()).map_err(|_| "Failed! Path string is Null")?;
        // let data = rusty_data.as_ptr() as *const c_void;
        let data = std::ptr::null();
        let meta = std::ptr::null();
        // let size = rusty_data.len();
        let size = 0;

        let game_info = libretro_sys::retro_game_info { data, meta, path: path.as_ptr(), size};
        
        
        if unsafe { core_api.retro_load_game(&game_info) } {
            Ok(InitialisedCoreWithRom { core_api, core_path, rom_path, frame_buffer: None, frame_format })  
        }
        else {
            Err("Failed to load rom!".to_string())
        }
    }
}

#[derive(Clone)]
struct InitialisedCoreWithRom {
    core_api: Arc<libretro_sys::LibretroAPI>,
    core_path: String,
    rom_path: String,
    frame_format: PixelFormat,
    frame_buffer: Option<FrameBuffer>
}

impl InitialisedCoreWithRom {
    fn unload_rom(self) -> InitialisedCore {
        let Self { core_api, core_path, frame_format, ..} = self;
        unsafe {
            core_api.retro_unload_game();
        }

        InitialisedCore { core_api, core_path, frame_format }
    }

    fn run(self) -> Self {
        unsafe {
            self.core_api.retro_run();
        }

        self
    }

    fn get_frame_info(&self) -> Result<FrameBuffer, String> {
        self.frame_buffer.clone().ok_or_else(|| "Frame buffer is empty".to_owned())
    }
}
