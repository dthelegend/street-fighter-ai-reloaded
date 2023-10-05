use std::{fs, ffi::{c_void, c_uint}};
use libffi::high::{ClosureMut4, ClosureMut3, ClosureMut2};
use libretro_sys::{CoreAPI, GameInfo};
use libloading::Library;

const EXPECTED_LIB_RETRO_VERSION: u32 = 1;

struct LibRetroEnvironment<'a> {
    dylib: Library,
    core_api: CoreAPI,
    pub core_path: String,
    pub rom_path: Option<String>,
    pub frame_buffer: Option<FrameBuffer>,
    svr_closure: Option<ClosureMut4<'a, *const c_void, c_uint, c_uint, usize, ()>>
    env_closure: Option<ClosureMut2<>>
}

struct FrameBuffer {
    frame_buffer: Vec<u8>,
    width: u32,
    height: u32,
}

impl LibRetroEnvironment<'_> {
    pub fn load_rom(&self, rom_path: &str) -> Result<(), String> {
        unsafe {
            let rusty_data = fs::read(rom_path).map_err(|err| format!("Failed to read reom into memory!\nReason: {}", err.kind()))?;
            
            let path = std::ffi::CString::new(rom_path).map_err(|_| "Failed! String is Null")?.as_ptr();
            let data = rusty_data.as_ptr() as *const c_void;
            let meta = std::ptr::null();
            let size = rusty_data.len();

            let game_info = GameInfo { data, meta, path, size};
            
            if !((self.core_api.retro_load_game)(&game_info)) {
                return Err("Failed to load rom!".to_string());
            }
        }

        self.rom_path = Some(rom_path.to_owned());

        Ok(())
    }

    pub fn init(&self) {
        let mut env = |command: u32, return_data: *mut c_void| -> i32 {
            match command {
                ENVIRONMENT_GET_CAN_DUPE => unsafe { *(return_data as *mut bool) = true },
                _ => println!("libretro_environment_callback Called with command: {}", command)
            }
        
            0
        };

        let env_closure: ClosureMut2<'_, u32, *mut c_void, i32> = ClosureMut2::new(&mut env);
        self.env_closure = Some(env_closure);
        let &env_code = env_closure.code_ptr();
        let env_ptr:unsafe extern "C" fn(*const std::ffi::c_void, u32, u32, usize) = unsafe { std::mem::transmute(env_code) };

        
        unsafe extern "C" fn libretro_set_input_poll_callback() {
            println!("libretro_set_input_poll_callback")
        }
        
        unsafe extern "C" fn libretro_set_input_state_callback<const environment_number: usize>(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16 {
            println!("libretro_set_input_state_callback");
            return 0; // Hard coded 0 for now means nothing is pressed
        }
        
        unsafe extern "C" fn libretro_set_audio_sample_callback<const environment_number: usize>(left: i16, right: i16) {
            println!("libretro_set_audio_sample_callback");
        }
        
        unsafe extern "C" fn libretro_set_audio_sample_batch_callback<const environment_number: usize>(data: *const i16, frames: usize) -> usize {
            println!("libretro_set_audio_sample_batch_callback");
            return 1;
        }

        let mut svr = |data: *const c_void, width: c_uint, height: c_uint, pitch: usize| {
            if data == std::ptr::null() {self.frame_buffer = None}
            else {
                unsafe {
                    self.frame_buffer = Some(FrameBuffer {
                        frame_buffer: Vec::from(std::slice::from_raw_parts(data as *const u8, (width * height) as usize)),
                        width,
                        height
                    })
                }
            };
        };

        let svr_closure = ClosureMut4::new(&mut svr);
        self.svr_closure = Some(svr_closure);
        let &svr_code = svr_closure.code_ptr();
        let svr_ptr:unsafe extern "C" fn(*const std::ffi::c_void, u32, u32, usize) = unsafe { std::mem::transmute(svr_code) };

        (self.core_api.retro_init)();
        (self.core_api.retro_set_video_refresh)(svr_ptr);
        
        // (self.core_api.retro_set_input_poll)(libretro_set_input_poll_callback::<id>);
        // (self.core_api.retro_set_input_state)(libretro_set_input_state_callback::<id>);
        // (self.core_api.retro_set_audio_sample)(libretro_set_audio_sample_callback::<id>);
        // (self.core_api.retro_set_audio_sample_batch)(libretro_set_audio_sample_batch_callback::<id>);
    }

    pub fn run(self) {
        (self.core_api.retro_run)();
    }


    pub fn new(core_path: &str) -> Result<LibRetroEnvironment, String>{
        unsafe {
            let dylib = Library::new(core_path).map_err(|_| "Failed to load Core")?;
        
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
                dylib,
                core_api,
                frame_buffer: None,
                rom_path: None,
                svr_closure: None,
            };

            return Ok(lib_retro_environment);
        }
    }
}
