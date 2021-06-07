use std::f32::consts::TAU;

use crate::prelude::*;

/// Contains methods for reading and writing room data.
/// Upon construction, comsumes the [Room](../room/struct.Room.html).
/// Should be packed into a [RwLock](std::sync::RwLock) using [new_thread_safe()](#method.new_thread_safe).
/// The RwLock's write lock should only be obtained by an [InputDevice](../devices/trait.InputDevice.html).
pub struct RoomController {
    pub room_data: RoomData,
    angle_dir_led_index_triplets: Vec<(f32, Vector2D, usize)>,
}

impl RoomController {
    /// Creates a RoomController by consuming room, and then wrap the RoomController for thread safety.
    pub fn new(filepath: &str) -> RoomController {
        let room_data = RoomData::new_from_file(filepath);

        let mut angle_dir_led_index_triplets: Vec<(f32, Vector2D, usize)> = vec![];

        let led_count = room_data.leds().len();
        let view = room_data.view_pos();

        for index in 0..led_count {
            let t = index as f32 / led_count as f32;
            let p = room_data.get_pos_at_t(t);
            let d = (p.0 - view.0, p.1 - view.1);
            let angle = d.1.atan2(d.0);
            angle_dir_led_index_triplets.push((
                (angle + TAU) % TAU,
                (angle.cos(), angle.sin()),
                index,
            ));
        }

        RoomController {
            room_data,
            angle_dir_led_index_triplets,
        }
    }

    /// Sets the color of a given led
    pub fn set(&mut self, index: usize, color: Color) {
        self.room_data.set_led(index, color);
    }

    /// Sets the color of all leds in the room
    pub fn set_all(&mut self, color: Color) {
        for index in 0..self.room_data.leds().len() {
            self.room_data.set_led(index, color);
        }
    }

    /// Sets the color of the pixel in a given direction, relative to the view.
    pub fn set_at_view_dir(&mut self, dir: Vector2D, color: Color) {
        self.set_at_room_dir(self.room_data.view_dir_to_room_dir(dir), color);
    }

    /// Sets the color of the pixel at a given angle, relative to the view.
    pub fn set_at_view_angle(&mut self, angle: f32, color: Color) {
        self.set_at_room_angle(self.room_data.view_angle_to_room_angle(angle), color);
    }

    /// Sets the color of the pixel at a given angle, relative to the room.
    pub fn set_at_room_angle(&mut self, angle: f32, color: Color) {
        let room_dir = (angle.cos(), angle.sin());
        self.set_at_room_dir(room_dir, color);
    }

    /// Casts a ray in the given direction, in room coordinate space, from the camera's position.
    /// If it hits a wall, the id of the led closest to that wall position will be returned, as well as the
    /// "Occupancy" of that led, where 1.0 means the ray lands directly on the LED, and 0.5 means the ray is halfway
    /// between that led and the next one up.
    pub fn get_led_at_room_dir(&self, dir: Vector2D) -> Option<(usize, f32)> {
        let view_pos = self.room_data.view_pos();
        let dist = 100.0;
        let ray_end = (view_pos.0 + (dir.0 * dist), view_pos.1 + (dir.1 * dist));
        let mut intersection: Option<Point> = None;
        let mut strip_index = 0;
        let mut led_count = 0.0;

        for strip in self.room_data.strips() {
            let i = strip.intersects(&(view_pos, ray_end));
            if i.is_some() {
                intersection = i;
                break;
            }
            strip_index += 1;
            led_count += strip.len() * self.room_data.density();
        }

        if intersection.is_none() {
            return None;
        }

        let strip = self.room_data.strips()[strip_index];
        let intersection_point = intersection.unwrap();
        let tx = reverse_lerp(strip.0, strip.1, intersection_point);
        led_count += tx * self.room_data.density() * strip.len();
        if led_count > 0.0 {
            led_count -= 1.0;
        }
        let occupancy = 1.0 - (led_count - led_count.floor());
        Some((led_count as usize, occupancy))
    }

    /// Returns the color of the led at the given room-space direction.
    /// If no led exists in that direction, black is returned.
    pub fn get_color_at_room_dir(&self, dir: Vector2D) -> Color {
        match self.get_led_at_room_dir(dir) {
            Some((id, occupancy)) => self.room_data.leds()[id],
            None => (0, 0, 0),
        }
    }

    /// Uses get_led_at_room_dir() to color an led at a given room-space direction.
    pub fn set_at_room_dir(&mut self, dir: Vector2D, color: Color) {
        if let Some((id, occupancy)) = self.get_led_at_room_dir(dir) {
            let c0 = (
                (color.0 as f32 * occupancy) as u8,
                (color.1 as f32 * occupancy) as u8,
                (color.2 as f32 * occupancy) as u8,
            );
            let next_occ = 1.0 - occupancy;
            let c1 = (
                (color.0 as f32 * next_occ) as u8,
                (color.1 as f32 * next_occ) as u8,
                (color.2 as f32 * next_occ) as u8,
            );

            self.set(id as usize, c0);
            if id + 1 < self.room_data.leds().len() {
                self.set(id as usize + 1, c1);
            }
        }
    }

    /// Allows the user to pass in a Color-returning function to calculate the color of each led, given its angle.
    pub fn map_angle_to_color(&mut self, map: &dyn Fn(f32) -> Color) {
        for (angle, _dir, led_index) in &self.angle_dir_led_index_triplets {
            let color = map(*angle);
            self.room_data.set_led(*led_index, color);
        }
    }

    /// Allows the user to pass in a Color-returning function to calculate the color of each led within a range, given its angle.
    pub fn map_angle_to_color_clamped(
        &mut self,
        map: &dyn Fn(f32) -> Color,
        min_angle: f32,
        max_angle: f32,
    ) {
        let adjusted_min = (min_angle + TAU) % TAU;
        let adjusted_max = (max_angle + TAU) % TAU;
        let crosses_wraparound = min_angle < 0.0 && max_angle > 0.0;

        for (angle, _dir, led_index) in &self.angle_dir_led_index_triplets {
            let deref_angle = *angle;
            // if this angle doesn't fit in the arc, skip it
            if crosses_wraparound {
                if !((deref_angle < TAU && deref_angle > adjusted_min)
                    || (deref_angle > 0.0 && deref_angle < adjusted_max))
                {
                    continue;
                }
            } else if !(deref_angle > adjusted_min && deref_angle < adjusted_max) {
                continue;
            }

            self.room_data.set_led(*led_index, map(deref_angle));
        }
    }

    /// Allows the user to pass in a Color-returning function to calculate the color of each led, given its direction.
    pub fn map_dir_to_color(&mut self, map: &dyn Fn(Vector2D) -> Color) {
        for (_angle, dir, led_index) in &self.angle_dir_led_index_triplets {
            let color = map(*dir);
            self.room_data.set_led(*led_index, color);
        }
    }

    /// Allows the user to pass in a Color-returning function to calculate the color of each led within an angle range, given its direction.
    pub fn map_dir_to_color_clamped(
        &mut self,
        map: &dyn Fn(Vector2D) -> Color,
        min_angle: f32,
        max_angle: f32,
    ) {
        let adjusted_min = (min_angle + TAU) % TAU;
        let adjusted_max = (max_angle + TAU) % TAU;
        let crosses_wraparound = min_angle < 0.0 && max_angle > 0.0;

        for (angle, dir, led_index) in &self.angle_dir_led_index_triplets {
            let deref_angle = *angle;
            // if this angle doesn't fit in the arc, skip it
            if crosses_wraparound {
                if !((deref_angle < TAU && deref_angle > adjusted_min)
                    || (deref_angle > 0.0 && deref_angle < adjusted_max))
                {
                    continue;
                }
            } else if !(deref_angle > adjusted_min && deref_angle < adjusted_max) {
                continue;
            }

            self.room_data.set_led(*led_index, map(*dir));
        }
    }
}

/// if lerp(a, b, t) = c, reverse_lerb(a, b, c) = t
fn reverse_lerp(a: Point, b: Point, c: Point) -> f32 {
    if a.0 != b.0 {
        (c.0 - a.0) / (b.0 - a.0)
    } else {
        (c.1 - a.1) / (b.1 - a.1)
    }
}
