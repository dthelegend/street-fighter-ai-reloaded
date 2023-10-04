use sdl2::{render::WindowCanvas, video::Window, Sdl, EventPump};
use super::libretrocore::LibRetroEnvironment;

struct GameStateManager {
    environment: LibRetroEnvironment
}

impl GameStateManager {
    fn new(core_path: &str, rom_path: &str) -> Result<GameStateManager, String> {
        let game_state_manager = GameStateManager {
            environment: LibRetroEnvironment::new(core_path)?
        };
        game_state_manager.environment.init();
        game_state_manager.environment.load_rom(rom_path)?;
        
        Ok(game_state_manager)
    }
}

struct GuiWindowManager {
    context: Sdl,
    window: Window,
    canvas: WindowCanvas,
    event_pump: EventPump
}

impl GuiWindowManager {
    fn new(game_name: &str) -> Result<GuiWindowManager, String> {
        let context = sdl2::init()?;
        let video_subsystem = context.video()?;

        let window = video_subsystem.window(game_name, 800, 600)
            .position_centered()
            .build()
            .expect("could not initialize video subsystem");

        let canvas = window.into_canvas()
            .build()
            .expect("could not make a canvas");
        
        let event_pump = context.event_pump()?;

        Ok(GuiWindowManager {
            context,
            window,
            canvas,
            event_pump
        })
    }

    fn render(&mut self, game_state: GameStateManager) {
        self.canvas.clear();
        self.canvas.present();
    }
}

enum GameManager {
    HeadlessGameManager(GameStateManager),
    GuiGameManager(GameStateManager, GuiWindowManager)
}

impl GameManager {
    pub fn new(is_headless: bool) -> GameManager {
        if is_headless {
            GameManager::HeadlessGameManager(GameStateManager::new())
        }
        else {
            GameManager::GuiGameManager(GameStateManager::new(), GuiWindowManager::new())
        }
    }
}