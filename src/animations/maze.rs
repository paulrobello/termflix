use super::Animation;
use crate::render::{Canvas, RenderMode};
use rand::RngExt;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Generating,
    Solving,
    Displaying,
}

#[derive(Clone, Copy, PartialEq)]
enum CellState {
    Unvisited,
    Visited,
    Explored,
    Solution,
}

/// Maze cell — tracks which walls are still standing
#[derive(Clone, Copy)]
struct Cell {
    walls: [bool; 4], // top, right, bottom, left
    state: CellState,
}

/// Animated maze generation with recursive backtracking and BFS solving
pub struct Maze {
    width: usize,
    height: usize,
    grid: Vec<Cell>,
    grid_w: usize,
    grid_h: usize,
    stack: Vec<(usize, usize)>,
    phase: Phase,
    solve_queue: Vec<(usize, usize)>,
    solve_parent: Vec<Option<(usize, usize)>>,
    solution_path: Vec<(usize, usize)>,
    solve_head: usize,
    display_timer: f64,
    steps_per_frame: usize,
    rng: rand::rngs::ThreadRng,
}

impl Maze {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let mut maze = Maze {
            width,
            height,
            grid: Vec::new(),
            grid_w: 0,
            grid_h: 0,
            stack: Vec::new(),
            phase: Phase::Generating,
            solve_queue: Vec::new(),
            solve_parent: Vec::new(),
            solution_path: Vec::new(),
            solve_head: 0,
            display_timer: 0.0,
            steps_per_frame: 3,
            rng: rand::rng(),
        };
        maze.build_grid();
        maze
    }

    fn build_grid(&mut self) {
        // Each maze cell maps to a 3x3 character area in the canvas:
        //   #.#    # = wall char, . = passage char
        //   ...    center row/col are the passage between cells
        //   #.#
        // So grid fits: grid_w = (width - 1) / 3, grid_h = (height - 1) / 3
        self.grid_w = if self.width >= 4 {
            (self.width - 1) / 3
        } else {
            1
        };
        self.grid_h = if self.height >= 4 {
            (self.height - 1) / 3
        } else {
            1
        };
        // Cap so the rendered area fits within canvas
        while self.grid_w * 3 + 1 > self.width && self.grid_w > 0 {
            self.grid_w -= 1;
        }
        while self.grid_h * 3 + 1 > self.height && self.grid_h > 0 {
            self.grid_h -= 1;
        }
        self.grid_w = self.grid_w.max(2);
        self.grid_h = self.grid_h.max(2);

        let total = self.grid_w * self.grid_h;
        self.grid = vec![
            Cell {
                walls: [true, true, true, true],
                state: CellState::Unvisited,
            };
            total
        ];

        // Start recursive backtracking from (0,0)
        self.grid[0].state = CellState::Visited;
        self.stack = vec![(0, 0)];

        self.phase = Phase::Generating;
        self.solve_queue = Vec::new();
        self.solve_parent = Vec::new();
        self.solution_path = Vec::new();
        self.solve_head = 0;
        self.display_timer = 0.0;

        // Pace generation: more cells = more steps per frame
        self.steps_per_frame = ((total as f64).sqrt() * 0.8).ceil() as usize;
        self.steps_per_frame = self.steps_per_frame.clamp(2, 40);
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.grid_w + x
    }

    fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.grid_w && y < self.grid_h
    }

    // Neighbors in order: top, right, bottom, left
    fn neighbors(&self, x: usize, y: usize) -> [(usize, usize); 4] {
        [
            (x, y.saturating_sub(1)), // top
            (x.saturating_add(1), y), // right
            (x, y.saturating_add(1)), // bottom
            (x.saturating_sub(1), y), // left
        ]
    }

    /// Remove walls between two adjacent cells
    fn remove_wall(&mut self, a: (usize, usize), b: (usize, usize)) {
        let dx = b.0 as isize - a.0 as isize;
        let dy = b.1 as isize - a.1 as isize;
        let ai = self.idx(a.0, a.1);
        let bi = self.idx(b.0, b.1);
        match (dx, dy) {
            (0, -1) => {
                // b is above a
                self.grid[ai].walls[0] = false;
                self.grid[bi].walls[2] = false;
            }
            (1, 0) => {
                // b is right of a
                self.grid[ai].walls[1] = false;
                self.grid[bi].walls[3] = false;
            }
            (0, 1) => {
                // b is below a
                self.grid[ai].walls[2] = false;
                self.grid[bi].walls[0] = false;
            }
            (-1, 0) => {
                // b is left of a
                self.grid[ai].walls[3] = false;
                self.grid[bi].walls[1] = false;
            }
            _ => {}
        }
    }

    fn step_generate(&mut self) -> bool {
        // Returns true when generation is complete
        while let Some(&(cx, cy)) = self.stack.last() {
            // Find unvisited neighbors
            let nbrs = self.neighbors(cx, cy);
            let mut unvisited: Vec<(usize, usize)> = Vec::new();
            for &(nx, ny) in &nbrs {
                if self.in_bounds(nx, ny)
                    && self.grid[self.idx(nx, ny)].state == CellState::Unvisited
                {
                    unvisited.push((nx, ny));
                }
            }

            if unvisited.is_empty() {
                self.stack.pop();
                // Continue backtracking — don't count as a step
                continue;
            }

            // Pick a random unvisited neighbor
            let pick = unvisited[self.rng.random_range(0..unvisited.len())];
            self.remove_wall((cx, cy), pick);
            let pick_idx = self.idx(pick.0, pick.1);
            self.grid[pick_idx].state = CellState::Visited;
            self.stack.push(pick);
            return false; // did one carving step
        }

        // Stack empty — maze fully generated
        true
    }

    fn begin_solve(&mut self) {
        let total = self.grid_w * self.grid_h;
        self.solve_parent = vec![None; total];
        self.solve_queue = vec![(0, 0)];
        self.solve_head = 0;
        self.grid[0].state = CellState::Explored;
        self.phase = Phase::Solving;
    }

    fn step_solve(&mut self) -> bool {
        // Returns true when solving is complete
        let steps = self.steps_per_frame * 2;
        for _ in 0..steps {
            if self.solve_head >= self.solve_queue.len() {
                // BFS done — reconstruct path
                self.reconstruct_path();
                return true;
            }

            let (cx, cy) = self.solve_queue[self.solve_head];
            self.solve_head += 1;

            if cx == self.grid_w - 1 && cy == self.grid_h - 1 {
                // Found the end
                self.reconstruct_path();
                return true;
            }

            let cell = self.grid[self.idx(cx, cy)];
            let nbrs = self.neighbors(cx, cy);

            // Check each direction — can only traverse if wall is removed
            for (dir, &(nx, ny)) in nbrs.iter().enumerate() {
                if !self.in_bounds(nx, ny) || cell.walls[dir] {
                    continue;
                }
                let ni = self.idx(nx, ny);
                if self.grid[ni].state == CellState::Explored {
                    continue;
                }
                self.grid[ni].state = CellState::Explored;
                self.solve_parent[ni] = Some((cx, cy));
                self.solve_queue.push((nx, ny));
            }
        }
        false
    }

    fn reconstruct_path(&mut self) {
        let mut path = Vec::new();
        let mut cur = (self.grid_w - 1, self.grid_h - 1);
        while let Some(parent) = self.solve_parent[self.idx(cur.0, cur.1)] {
            path.push(cur);
            cur = parent;
        }
        path.push((0, 0));
        path.reverse();
        for &(px, py) in &path {
            let idx = self.idx(px, py);
            self.grid[idx].state = CellState::Solution;
        }
        self.solution_path = path;
        self.phase = Phase::Displaying;
        self.display_timer = 0.0;
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.clear();

        let cw = canvas.width;
        let ch = canvas.height;

        // Wall colors
        let wall_r: u8 = 40;
        let wall_g: u8 = 50;
        let wall_b: u8 = 90;

        // Passage (visited) color
        let path_r: u8 = 180;
        let path_g: u8 = 200;
        let path_b: u8 = 220;

        // Explored (BFS visited) color
        let expl_r: u8 = 50;
        let expl_g: u8 = 70;
        let expl_b: u8 = 120;

        // Solution path color
        let sol_r: u8 = 80;
        let sol_g: u8 = 255;
        let sol_b: u8 = 120;

        // Start marker
        let start_r: u8 = 100;
        let start_g: u8 = 200;
        let start_b: u8 = 255;

        // End marker
        let end_r: u8 = 255;
        let end_g: u8 = 200;
        let end_b: u8 = 60;

        // Stack head (current cell during generation) color
        let cur_r: u8 = 255;
        let cur_g: u8 = 100;
        let cur_b: u8 = 50;

        for gy in 0..self.grid_h {
            for gx in 0..self.grid_w {
                let cell = &self.grid[self.idx(gx, gy)];
                let base_x = gx * 3;
                let base_y = gy * 3;

                // Center of the cell
                let (cr, cg, cb) = match cell.state {
                    CellState::Unvisited => (wall_r, wall_g, wall_b),
                    CellState::Visited => (path_r, path_g, path_b),
                    CellState::Explored => (expl_r, expl_g, expl_b),
                    CellState::Solution => (sol_r, sol_g, sol_b),
                };

                // Highlight current generation position
                let is_stack_head =
                    self.phase == Phase::Generating && self.stack.last() == Some(&(gx, gy));

                let (center_r, center_g, center_b, center_ch) = if is_stack_head {
                    (cur_r, cur_g, cur_b, '@')
                } else if gx == 0 && gy == 0 {
                    (start_r, start_g, start_b, 'S')
                } else if gx == self.grid_w - 1 && gy == self.grid_h - 1 {
                    (end_r, end_g, end_b, 'E')
                } else if cell.state == CellState::Solution {
                    (cr, cg, cb, 'o')
                } else {
                    (cr, cg, cb, ' ')
                };

                // Draw cell center
                if base_x + 1 < cw && base_y + 1 < ch {
                    canvas.set_char(
                        base_x + 1,
                        base_y + 1,
                        center_ch,
                        center_r,
                        center_g,
                        center_b,
                    );
                }

                // Top-left corner — always a wall pillar
                if base_x < cw && base_y < ch {
                    canvas.set_char(base_x, base_y, '#', wall_r, wall_g, wall_b);
                }

                // Top wall (between this cell and the one above)
                if cell.walls[0] {
                    if base_x + 1 < cw && base_y < ch {
                        canvas.set_char(base_x + 1, base_y, '-', wall_r, wall_g, wall_b);
                    }
                } else {
                    // Passage open — draw passage char with neighbor's color
                    let (pr, pg, pb) = if gy > 0 {
                        let above = &self.grid[self.idx(gx, gy - 1)];
                        match above.state {
                            CellState::Unvisited => (wall_r, wall_g, wall_b),
                            CellState::Visited => (path_r, path_g, path_b),
                            CellState::Explored => (expl_r, expl_g, expl_b),
                            CellState::Solution => (sol_r, sol_g, sol_b),
                        }
                    } else {
                        (path_r, path_g, path_b)
                    };
                    // Blend with current cell
                    let br = ((pr as u16 + cr as u16) / 2) as u8;
                    let bg = ((pg as u16 + cg as u16) / 2) as u8;
                    let bb = ((pb as u16 + cb as u16) / 2) as u8;
                    if base_x + 1 < cw && base_y < ch {
                        canvas.set_char(base_x + 1, base_y, ' ', br, bg, bb);
                    }
                }

                // Left wall (between this cell and the one to the left)
                if cell.walls[3] {
                    if base_x < cw && base_y + 1 < ch {
                        canvas.set_char(base_x, base_y + 1, '|', wall_r, wall_g, wall_b);
                    }
                } else {
                    let (pr, pg, pb) = if gx > 0 {
                        let left = &self.grid[self.idx(gx - 1, gy)];
                        match left.state {
                            CellState::Unvisited => (wall_r, wall_g, wall_b),
                            CellState::Visited => (path_r, path_g, path_b),
                            CellState::Explored => (expl_r, expl_g, expl_b),
                            CellState::Solution => (sol_r, sol_g, sol_b),
                        }
                    } else {
                        (path_r, path_g, path_b)
                    };
                    let br = ((pr as u16 + cr as u16) / 2) as u8;
                    let bg = ((pg as u16 + cg as u16) / 2) as u8;
                    let bb = ((pb as u16 + cb as u16) / 2) as u8;
                    if base_x < cw && base_y + 1 < ch {
                        canvas.set_char(base_x, base_y + 1, ' ', br, bg, bb);
                    }
                }
            }
        }

        // Draw bottom border wall
        let bottom_y = self.grid_h * 3;
        if bottom_y < ch {
            for gx in 0..self.grid_w {
                let cell = &self.grid[self.idx(gx, self.grid_h - 1)];
                let bx = gx * 3;
                // Corner
                if bx < cw {
                    canvas.set_char(bx, bottom_y, '#', wall_r, wall_g, wall_b);
                }
                // Bottom wall of last row — only draw if wall is still there
                if cell.walls[2] {
                    if bx + 1 < cw {
                        canvas.set_char(bx + 1, bottom_y, '-', wall_r, wall_g, wall_b);
                    }
                } else {
                    let (pr, pg, pb) = match cell.state {
                        CellState::Solution => (sol_r, sol_g, sol_b),
                        CellState::Explored => (expl_r, expl_g, expl_b),
                        CellState::Visited => (path_r, path_g, path_b),
                        CellState::Unvisited => (wall_r, wall_g, wall_b),
                    };
                    if bx + 1 < cw {
                        canvas.set_char(bx + 1, bottom_y, ' ', pr, pg, pb);
                    }
                }
            }
        }

        // Draw right border wall
        let right_x = self.grid_w * 3;
        if right_x < cw {
            for gy in 0..self.grid_h {
                let cell = &self.grid[self.idx(self.grid_w - 1, gy)];
                let by = gy * 3;
                // Corner
                if by < ch {
                    canvas.set_char(right_x, by, '#', wall_r, wall_g, wall_b);
                }
                // Right wall of last column
                if cell.walls[1] {
                    if by + 1 < ch {
                        canvas.set_char(right_x, by + 1, '|', wall_r, wall_g, wall_b);
                    }
                } else {
                    let (pr, pg, pb) = match cell.state {
                        CellState::Solution => (sol_r, sol_g, sol_b),
                        CellState::Explored => (expl_r, expl_g, expl_b),
                        CellState::Visited => (path_r, path_g, path_b),
                        CellState::Unvisited => (wall_r, wall_g, wall_b),
                    };
                    if by + 1 < ch {
                        canvas.set_char(right_x, by + 1, ' ', pr, pg, pb);
                    }
                }
            }
            // Bottom-right corner
            if bottom_y < ch {
                canvas.set_char(right_x, bottom_y, '#', wall_r, wall_g, wall_b);
            }
        }
    }
}

impl Animation for Maze {
    fn name(&self) -> &str {
        "maze"
    }

    fn preferred_render(&self) -> RenderMode {
        RenderMode::HalfBlock
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.build_grid();
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        match self.phase {
            Phase::Generating => {
                for _ in 0..self.steps_per_frame {
                    if self.step_generate() {
                        self.begin_solve();
                        break;
                    }
                }
            }
            Phase::Solving => {
                self.step_solve();
            }
            Phase::Displaying => {
                self.display_timer += dt;
                if self.display_timer >= 4.0 {
                    self.build_grid();
                }
            }
        }

        self.draw(canvas);
    }
}
