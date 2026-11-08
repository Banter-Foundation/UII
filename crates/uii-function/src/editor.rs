use crate::{Camera, Direction, EditError, FunctionId, FunctionKind, Graph, Hit, PortId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Primary,
    Middle,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkPreview {
    pub from: FunctionId,
    pub from_port: PortId,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
enum Drag {
    Pan {
        last_x: f32,
        last_y: f32,
    },
    Node {
        id: FunctionId,
        grab_x: f32,
        grab_y: f32,
    },
    Link {
        from: FunctionId,
        from_port: PortId,
        x: f32,
        y: f32,
    },
}

/// Canvas gestures for the Function tab. Coordinates are canvas pixels,
/// origin at the top-left of the graph view.
#[derive(Debug)]
pub struct Editor {
    graph: Graph,
    camera: Camera,
    selected: Option<FunctionId>,
    drag: Option<Drag>,
    last_error: Option<EditError>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        Self::with_graph(Graph::new())
    }

    pub fn with_graph(graph: Graph) -> Self {
        Self {
            graph,
            camera: Camera::new(),
            selected: None,
            drag: None,
            last_error: None,
        }
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn selected(&self) -> Option<FunctionId> {
        self.selected
    }

    pub fn select(&mut self, id: Option<FunctionId>) {
        self.selected = id;
    }

    pub fn last_error(&self) -> Option<EditError> {
        self.last_error
    }

    pub fn linking(&self) -> Option<LinkPreview> {
        match self.drag {
            Some(Drag::Link {
                from,
                from_port,
                x,
                y,
            }) => Some(LinkPreview {
                from,
                from_port,
                x,
                y,
            }),
            _ => None,
        }
    }

    pub fn retain_selection(&mut self) {
        if let Some(id) = self.selected {
            if self.graph.get(id).is_none() {
                self.selected = None;
            }
        }
    }

    pub fn add_function_at(
        &mut self,
        canvas_x: f32,
        canvas_y: f32,
        kind: FunctionKind,
    ) -> FunctionId {
        let (x, y) = self.camera.canvas_to_world(canvas_x, canvas_y);
        let id = self.graph.create(kind, "", "", x, y);
        self.selected = Some(id);
        self.last_error = None;
        id
    }

    pub fn delete_selection(&mut self) {
        if let Some(id) = self.selected.take() {
            let _ = self.graph.delete(id);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    pub fn pointer_down(&mut self, x: f32, y: f32, button: Button, alt: bool) {
        if button == Button::Secondary {
            return;
        }
        self.last_error = None;
        if button == Button::Middle || (button == Button::Primary && alt) {
            self.drag = Some(Drag::Pan {
                last_x: x,
                last_y: y,
            });
            return;
        }
        let (wx, wy) = self.camera.canvas_to_world(x, y);
        match self.graph.hit_test(wx, wy) {
            Some(Hit::Port { function, port }) => {
                self.selected = Some(function);
                let direction = self
                    .graph
                    .get(function)
                    .and_then(|item| item.ports().find(|item| item.id == port))
                    .map(|item| item.direction);
                if direction == Some(Direction::Out) {
                    self.drag = Some(Drag::Link {
                        from: function,
                        from_port: port,
                        x: wx,
                        y: wy,
                    });
                } else {
                    self.drag = None;
                }
            }
            Some(Hit::Body { function }) => {
                self.selected = Some(function);
                let (nx, ny) = self.graph.get(function).unwrap().position();
                self.drag = Some(Drag::Node {
                    id: function,
                    grab_x: wx - nx,
                    grab_y: wy - ny,
                });
            }
            None => {
                self.selected = None;
                self.drag = None;
            }
        }
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) {
        match self.drag {
            Some(Drag::Pan { last_x, last_y }) => {
                self.camera.pan_by(x - last_x, y - last_y);
                self.drag = Some(Drag::Pan {
                    last_x: x,
                    last_y: y,
                });
            }
            Some(Drag::Node { id, grab_x, grab_y }) => {
                let (wx, wy) = self.camera.canvas_to_world(x, y);
                let _ = self.graph.move_to(id, wx - grab_x, wy - grab_y);
            }
            Some(Drag::Link {
                from, from_port, ..
            }) => {
                let (wx, wy) = self.camera.canvas_to_world(x, y);
                self.drag = Some(Drag::Link {
                    from,
                    from_port,
                    x: wx,
                    y: wy,
                });
            }
            None => {}
        }
    }

    pub fn pointer_up(&mut self, x: f32, y: f32, button: Button) {
        if button == Button::Secondary {
            return;
        }
        let drag = self.drag.take();
        if button != Button::Primary {
            return;
        }
        let Some(Drag::Link {
            from, from_port, ..
        }) = drag
        else {
            return;
        };
        let (wx, wy) = self.camera.canvas_to_world(x, y);
        let Some(Hit::Port { function, port }) = self.graph.hit_test(wx, wy) else {
            return;
        };
        match self.graph.connect(from, from_port, function, port) {
            Ok(_) => self.last_error = None,
            Err(err) => self.last_error = Some(err),
        }
    }

    pub fn scroll(&mut self, x: f32, y: f32, wheel_y: f32) {
        self.camera.zoom_at(x, y, wheel_y);
    }
}
