/// Pan and zoom for the function canvas. `pan` is the world point at the
/// canvas origin. `zoom` is canvas pixels per world pixel.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn pan(&self) -> (f32, f32) {
        (self.pan_x, self.pan_y)
    }

    pub fn set_view(&mut self, pan_x: f32, pan_y: f32, zoom: f32) {
        self.pan_x = pan_x;
        self.pan_y = pan_y;
        self.zoom = zoom.clamp(0.25, 4.0);
    }

    pub fn world_to_canvas(&self, wx: f32, wy: f32) -> (f32, f32) {
        ((wx - self.pan_x) * self.zoom, (wy - self.pan_y) * self.zoom)
    }

    pub fn canvas_to_world(&self, cx: f32, cy: f32) -> (f32, f32) {
        (cx / self.zoom + self.pan_x, cy / self.zoom + self.pan_y)
    }

    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        self.pan_x -= dx / self.zoom;
        self.pan_y -= dy / self.zoom;
    }

    pub fn zoom_at(&mut self, cx: f32, cy: f32, wheel_y: f32) {
        if wheel_y == 0.0 {
            return;
        }
        let (wx, wy) = self.canvas_to_world(cx, cy);
        let next = (self.zoom * 1.1_f32.powf(wheel_y)).clamp(0.25, 4.0);
        if next == self.zoom {
            return;
        }
        self.zoom = next;
        self.pan_x = wx - cx / self.zoom;
        self.pan_y = wy - cy / self.zoom;
    }
}
