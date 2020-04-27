use rand;

use tetra::{self, State, Context, ContextBuilder};
use tetra::audio;
use tetra::graphics::{self, Color, DrawParams, Font, Texture, Rectangle};
use tetra::math::Vec2;
use tetra::input::{self,Key};
use std::result;

const SHOT_SPEED: f32 = 200.0;

// Accleration in pixels per second.
const PLAYER_THRUST: f32 = 100.0;
// Rotation in radians per second.
const PLAYER_TURN_RATE: f32 = 3.0;
// Seconds between shots
const PLAYER_SHOT_TIME: f32 = 0.5;

const PLAYER_LIFE: f32 = 1.0;
const SHOT_LIFE: f32 = 2.0;
const ROCK_LIFE: f32 = 1.0;

const MAX_ROCK_VEL: f32 = 50.0;

type Point2 = Vec2<f32>;
type Vector2 = Vec2<f32>;

// normalized
fn vec_from_angle(angle: f32) -> Vector2 {
    let vx = angle.sin();
    let vy = angle.cos();
    Vector2::new(vx, vy)
}

// rand::random::<f32>() range (0,1)
fn random_vec(max_magnitude: f32) -> Vector2 {
    let angle = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
    let mag = rand::random::<f32>() * max_magnitude;
    vec_from_angle(angle) * (mag)
}

#[derive(Debug)]
struct Actor {
    texture: Texture,
    pos: Point2,
    facing: f32,
    velocity: Vector2,

    // lazily overload "life" with a
    // double meaning
    // for  shots, it is the time left to live,
    // for players and rocks, it is the actual hit points.
    life: f32,
}

impl Actor {
    fn width(&self) -> f32 {
        self.texture.width() as f32
    }
    fn height(&self) -> f32 {
        self.texture.height() as f32
    }

    fn origin(&self) -> Vec2<f32> {
        Vec2::new (
            self.width() / 2.0,
            self.height() / 2.0,
        )
    }

    fn create_player(ctx: &mut Context) -> tetra::Result<Actor> {
        let player_texture = Texture::new(ctx, "./resources/player.png")?;
        Ok( Actor {
            texture: player_texture,
            pos: Vec2::zero(),
            facing: 0.0,
            velocity: Vec2::zero(),
            life: PLAYER_LIFE,
        })
    }
    
    fn create_rock(ctx: &mut Context) -> tetra::Result<Actor> {
        let rock_texture = Texture::new(ctx, "./resources/rock.png")?;
        Ok(Actor {
            texture: rock_texture,
            pos: Vec2::zero(),
            facing: 0.0,
            velocity: Vec2::zero(),
            life: ROCK_LIFE,
        })
    }

    fn create_shot(ctx: &mut Context) -> tetra::Result<Actor> {
        let shot_texture = Texture::new(ctx, "./resources/shot.png")?;
        Ok( Actor {
            texture: shot_texture,
            pos: Vec2::zero(),
            facing: 0.,
            velocity: Vec2::zero(),
            life: SHOT_LIFE,
        })
    }

    fn player_thrust(actor: &mut Self, dt: f32) {
        let direction_vector = vec_from_angle(actor.facing);
        let thrust_vector = direction_vector * (PLAYER_THRUST);
        actor.velocity += thrust_vector * (dt);
    }

    fn update_actor_position(actor: &mut Self, dt: f32) {
        // Clamp the velocity to the max efficiency
        // The  velocity clamping  is  used  to  prevent  the  particles  from  rapid acceleration. 
        let mag_sq = actor.velocity.magnitude_squared();
        if mag_sq > MAX_PHYSICS_VEL.powi(2) {
            actor.velocity = actor.velocity / mag_sq.sqrt() * MAX_PHYSICS_VEL;
        }
        let dv = actor.velocity * (dt);
        actor.pos += dv;
    }

    /// Takes an actor and wraps its position to the bounds of the
    /// screen, so if it goes off the left side of the screen it
    /// will re-enter on the right side and so on.
    fn wrap_actor_position(actor: &mut Self, sx: f32, sy: f32) {
        // Wrap screen
        let screen_x_bounds = sx / 2.0;
        let screen_y_bounds = sy / 2.0;
        if actor.pos.x > screen_x_bounds {
            actor.pos.x -= sx;
        } else if actor.pos.x < -screen_x_bounds {
            actor.pos.x += sx;
        };
        if actor.pos.y > screen_y_bounds {
            actor.pos.y -= sy;
        } else if actor.pos.y < -screen_y_bounds {
            actor.pos.y += sy;
        }
    }

    // Decrease actor life with time.
    fn handle_timed_life(actor: &mut Self, dt: f32) {
        actor.life -= dt;
    }
}

/// Create the given number of rocks.
/// Make sure that none of them are within the
/// given exclusion zone (nominally the player)

/// Note the this *could* create rocks outside the
/// bounds of the playing field, so it should be 
/// called before 'wrap_actor_position()' happens.

// Params: num - num. of rocks to generate
// min_radius, max_radius - radius range for rocks.
fn create_rocks(ctx: &mut Context, num: i32, exclusion: Point2, min_radius: f32, max_radius: f32) -> Vec<Actor> {
    let mut new_rock = || -> tetra::Result<Actor> {
        assert!(max_radius > min_radius);
        let mut rock : Actor = Actor::create_rock(ctx)?;
        //random angle
        let r_angle = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
        let r_distance = rand::random::<f32>() * (max_radius - min_radius) + min_radius;
        // rock positioned wrt player
        rock.pos = exclusion + vec_from_angle(r_angle) * r_distance;
        rock.velocity = random_vec(MAX_ROCK_VEL);
        Ok(rock)
    };
    //let x = new_rock.ok();
    /// new rock a type of function, baad mein call kar dene ka.
    let mut vec : Vec<Actor> = Vec::new();
    let mut count = 0i32;
    loop {
        if count == num {
            break;
        }
        let y = new_rock().ok();
        match y {
            Some(x) => vec.push(x),
            None => panic!("Error at rock creation?"),
        }
        count += 1;
    }
    //(0..num).map(new_rock().ok()).collect()
    //let b : Result<Vec<_>, _>= (0..num).map(new_rock).collect();
    //let mut iter = b.iter();
    //iter.filter_map(|x| x.ok());

    //b.iter().try_fold(0, num)
    /*match result {
        Ok(val) => val,
    }*/
    vec
}

// GameState Input Struct
#[derive(Debug)]
struct InputState {
    xaxis: f32,
    yaxis: f32,
    fire: bool,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            xaxis: 0.0, // Left in rotation
            yaxis: 0.0, // Thrust of ship
            fire: false,
        }
    }
}

fn player_handle_input(actor: &mut Actor, input: &InputState, dt: f32) {
    actor.facing += dt * PLAYER_TURN_RATE * input.xaxis;

    if input.yaxis > 0.0 {
        Actor::player_thrust(actor, dt);
    }
}

const MAX_PHYSICS_VEL: f32 = 250.0;

/// Translates the world coordinate system, which
/// has Y pointing up and the origin at the center,
/// to the screen coordinate system, which has Y
/// pointing downward and the origin at the top-left,
fn world_to_screen_coords(screen_width: f32, screen_height: f32, point: Point2) -> Point2 {
    let x = point.x + screen_width / 2.0;
    let y = screen_height - (point.y + screen_height / 2.0);
    Point2::new(x,y)
} 

struct Assets {
    shot_sound: audio::Sound,
    hit_sound: audio::Sound,
}

impl Assets {
    fn new(ctx: &mut Context) -> tetra::Result<Assets> {

        let shot_sound = audio::Sound::new("./resources/pew.wav")?;
        let hit_sound = audio::Sound::new("./resources/boom.flac")?;

        Ok(Assets {
            shot_sound,
            hit_sound,
        })
    }
}

struct GameState {
    player: Actor,
    shots: Vec<Actor>,
    rocks: Vec<Actor>,
    level: i32,
    score: i32,
    assets: Assets,
    screen_width: f32,
    screen_height: f32,
    input: InputState,
    player_shot_timeout: f32,
}

impl GameState {
    fn new(ctx: &mut Context) -> tetra::Result<GameState> {
        print_instruction();

        let assets = Assets::new(ctx)?;
        let player = Actor::create_player(ctx)?;
        let rocks = create_rocks(ctx, 5, player.pos, 100.0, 250.0);

        let s = GameState {
            player,
            shots: Vec::new(),
            rocks,
            level: 0,
            score: 0,
            assets,
            screen_width: tetra::window::get_width(ctx) as f32,
            screen_height: tetra::window::get_height(ctx) as f32,
            input: InputState::default(),
            player_shot_timeout: 0.0,
        };

        Ok(s)
    }
    
    fn fire_player_shot(&mut self, ctx: &mut Context) -> tetra::Result{
        self.player_shot_timeout = PLAYER_SHOT_TIME;
    
        let player = &self.player;
        let mut shot = Actor::create_shot(ctx)?;
        shot.pos = player.pos;
        shot.facing = player.facing;
    
        let direction = vec_from_angle(shot.facing);
        shot.velocity.x = SHOT_SPEED * direction.x;
        shot.velocity.y = SHOT_SPEED * direction.y;
    
        self.shots.push(shot);
        let _ = self.assets.shot_sound.play(ctx);

        Ok(())
    }
    
    fn clear_dead_stuff(&mut self) {
        self.shots.retain(|s| s.life > 0.0);
        self.rocks.retain(|r| r.life > 0.0);
    }
    
    fn handle_collision(&mut self, ctx: &Context) {
        for rock in &mut self.rocks {
            let bound_rock = Rectangle::new (
                rock.pos.x,
                rock.pos.y,
                rock.width(),
                rock.height(),
            );
            let bound_player = Rectangle::new (
                self.player.pos.x,
                self.player.pos.y,
                self.player.width(),
                self.player.height(),
            );
            if bound_rock.intersects(&bound_player) {
                self.player.life = 0.0;
            };
            for shot in &mut self.shots {
                let bound_shot = Rectangle::new (
                    shot.pos.x,
                    shot.pos.y,
                    shot.width(),
                    shot.height(),
                );
                if bound_rock.intersects(&bound_shot)
                {
                    shot.life = 0.0;
                    rock.life = 0.0;
                    self.score += 1;

                    let _ = self.assets.hit_sound.play(ctx);
                }
            }
        }
    }

    fn check_for_level_respawn(&mut self, ctx: &mut Context) {
        if self.rocks.is_empty() {
            self.level += 1;
            let r = create_rocks(ctx, self.level + 5, self.player.pos, 100.0, 250.0);
            self.rocks.extend(r);
        }
    }
}

/// ***************************************************
/// A couple of utility functions.
/// ***************************************************

fn print_instruction() {
    println!();
    println!("Welcome to ASTROBLASTO!");
    println!();
    println!("How to play:");
    println!("L/R arrow keys to rotate your ship, up thrusts, space bar fires");
    println!();
}

fn draw_actor(
    ctx: &mut Context,
    actor: &Actor,
    world_coords: (f32, f32),
) -> tetra::Result {
    let (screen_w, screen_h) = world_coords;
    let pos = world_to_screen_coords(screen_w, screen_h, actor.pos);
    let image = &actor.texture;
    let drawparams = graphics::DrawParams::new()
        .position(pos)
        .rotation(actor.facing as f32)
        .origin(Point2::new(0.5, 0.5));
    graphics::draw(ctx, image, drawparams);
    Ok(())
}

impl State for GameState {
    
    fn update(&mut self, ctx: &mut Context) -> tetra::Result {
        const DESIRED_FPS : u32 = 60;
        let seconds = 1.0 / (DESIRED_FPS as f32);
        // Update the player state based on the user input.
        self.input.xaxis = 
            if input::is_key_down(ctx, Key::Left) {
                -1.0
            } else if input::is_key_down(ctx, Key::Right) {
                1.0
            } 
            else {
                0.
            };
        self.input.yaxis = 
            if input::is_key_down(ctx, Key::Up) {
                1.0
            } else {
                0.
            };
        self.input.fire = 
            if input::is_key_down(ctx, Key::Space) {
                true
            } else {
                false
            };
        player_handle_input(&mut self.player, &self.input, seconds);
        self.player_shot_timeout -= seconds;
        if self.input.fire && self.player_shot_timeout < 0.0 {
            self.fire_player_shot(ctx);
        }
        if self.input.yaxis != 0.{
            //self.player
        }

        //Update the physics for all actors.
        // First the player...
        Actor::update_actor_position(&mut self.player, seconds);
        Actor::wrap_actor_position(
            &mut self.player,
            self.screen_width as f32,
            self.screen_height as f32,
        );

        // Then the shots...
        for act in &mut self.shots {
            Actor::update_actor_position(act, seconds);
            Actor::wrap_actor_position(act, self.screen_width as f32, self.screen_height as f32);
            Actor::handle_timed_life(act, seconds);
        }

        // Handle the results of things moving:
        // collision detection, object death, and if
        // we have killed all the rocks in the level,
        // spawn more of them.
        self.handle_collision(ctx);

        self.clear_dead_stuff();

        self.check_for_level_respawn(ctx);

        // Finally we check for our end state
        // I wnat to have a nice death screen eventually,
        // but for now we just quit
        if self.player.life <= 0.0 {
            println!("Game over!");
            tetra::window::quit(ctx);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> tetra::Result {
        graphics::clear(ctx, Color::rgb(0.0, 0.0, 0.0));

        // Loop over all objects drawing them...
        {
            let coords = (self.screen_width, self.screen_height);

            let p = &self.player;
            draw_actor(ctx, p, coords)?;

            for s in &self.shots {
                draw_actor(ctx, s, coords)?;
            }

            for r in &self.rocks {
                draw_actor(ctx, r, coords)?;
            }
        }

        // and draw the GUI elements in the right places.
        let level_dest = Point2::new(10.0, 10.0);
        let score_dest = Point2::new(200.0, 10.0);

        let level_str = format!("Level: {}", self.level);
        let score_str = format!("Score: {}", self.score);
        let level_display = graphics::Text::new(level_str, Font::default(), 32.0);
        let score_display = graphics::Text::new(score_str, Font::default(), 32.0);
        graphics::draw(ctx, &level_display, DrawParams::new().position(level_dest));
        graphics::draw(ctx, &score_display, DrawParams::new().position(score_dest));

        // And yield the timeline
        // This tells the OS that we're done using the CPU but it should
        // get back to this program as soon as it can.
        // This ideally prevents the game from using 100% CPU all the time
        // even if vync is off
        // The actual behavior can be a little platform specific.
        Ok(())
    }
}

pub fn main() -> tetra::Result {
    ContextBuilder::new("Tetra Astroblasto", 800, 600)
        .quit_on_escape(true)
        .build()?
        .run(GameState::new)
}