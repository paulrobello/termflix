use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

#[derive(Clone, Copy, PartialEq)]
#[allow(clippy::enum_variant_names)]
enum SortAlgo {
    QuickSort,
    MergeSort,
    HeapSort,
}

impl SortAlgo {
    fn name(self) -> &'static str {
        match self {
            SortAlgo::QuickSort => "quicksort",
            SortAlgo::MergeSort => "mergesort",
            SortAlgo::HeapSort => "heapsort",
        }
    }

    fn next(self) -> Self {
        match self {
            SortAlgo::QuickSort => SortAlgo::MergeSort,
            SortAlgo::MergeSort => SortAlgo::HeapSort,
            SortAlgo::HeapSort => SortAlgo::QuickSort,
        }
    }
}

/// Sorting algorithm visualizer cycling through quicksort, mergesort, heapsort
pub struct Sort {
    data: Vec<f64>,
    ops: Vec<SortOp>,
    op_index: usize,
    algo: SortAlgo,
    active_indices: (usize, usize),
    sorted: bool,
    pause_timer: f64,
    ops_per_frame: usize,
    rng: rand::rngs::ThreadRng,
}

#[derive(Clone, Copy)]
enum SortOp {
    Compare(usize, usize),
    Swap(usize, usize),
}

impl Sort {
    pub fn new(width: usize, _height: usize, _scale: f64) -> Self {
        let size = (width / 2).clamp(16, 200);
        let mut rng = rand::rng();
        let data: Vec<f64> = (0..size).map(|_| rng.random_range(0.05..1.0)).collect();

        let mut sort = Sort {
            data,
            ops: Vec::new(),
            op_index: 0,
            algo: SortAlgo::QuickSort,
            active_indices: (0, 0),
            sorted: false,
            pause_timer: 0.0,
            ops_per_frame: 3,
            rng: rand::rng(),
        };
        sort.generate_ops();
        sort
    }

    fn generate_ops(&mut self) {
        self.ops.clear();
        self.op_index = 0;
        self.sorted = false;

        let mut data = self.data.clone();
        let mut ops = Vec::new();

        match self.algo {
            SortAlgo::QuickSort => {
                let hi = data.len().saturating_sub(1) as isize;
                quicksort(&mut data, 0, hi, &mut ops);
            }
            SortAlgo::MergeSort => {
                let len = data.len();
                mergesort(&mut data, 0, len, &mut ops);
            }
            SortAlgo::HeapSort => heapsort(&mut data, &mut ops),
        }

        self.ops = ops;
    }

    fn shuffle(&mut self) {
        let n = self.data.len();
        for i in (1..n).rev() {
            let j = self.rng.random_range(0..=i);
            self.data.swap(i, j);
        }
        self.generate_ops();
    }
}

fn quicksort(data: &mut [f64], low: isize, high: isize, ops: &mut Vec<SortOp>) {
    if low < high {
        let pivot = partition(data, low as usize, high as usize, ops);
        quicksort(data, low, pivot as isize - 1, ops);
        quicksort(data, pivot as isize + 1, high, ops);
    }
}

fn partition(data: &mut [f64], low: usize, high: usize, ops: &mut Vec<SortOp>) -> usize {
    let pivot = data[high];
    let mut i = low;
    for j in low..high {
        ops.push(SortOp::Compare(j, high));
        if data[j] <= pivot {
            ops.push(SortOp::Swap(i, j));
            data.swap(i, j);
            i += 1;
        }
    }
    ops.push(SortOp::Swap(i, high));
    data.swap(i, high);
    i
}

fn mergesort(data: &mut [f64], left: usize, right: usize, ops: &mut Vec<SortOp>) {
    if right - left <= 1 {
        return;
    }
    let mid = (left + right) / 2;
    mergesort(data, left, mid, ops);
    mergesort(data, mid, right, ops);
    merge(data, left, mid, right, ops);
}

fn merge(data: &mut [f64], left: usize, mid: usize, right: usize, ops: &mut Vec<SortOp>) {
    let merged: Vec<f64> = {
        let mut result = Vec::new();
        let mut i = left;
        let mut j = mid;
        while i < mid && j < right {
            ops.push(SortOp::Compare(i, j));
            if data[i] <= data[j] {
                result.push(data[i]);
                i += 1;
            } else {
                result.push(data[j]);
                j += 1;
            }
        }
        while i < mid {
            result.push(data[i]);
            i += 1;
        }
        while j < right {
            result.push(data[j]);
            j += 1;
        }
        result
    };

    for (k, &val) in merged.iter().enumerate() {
        let idx = left + k;
        if (data[idx] - val).abs() > f64::EPSILON {
            ops.push(SortOp::Swap(idx, idx)); // visual indicator of write
        }
        data[idx] = val;
    }
}

fn heapsort(data: &mut [f64], ops: &mut Vec<SortOp>) {
    let n = data.len();

    // Build max heap
    for i in (0..n / 2).rev() {
        heapify(data, n, i, ops);
    }

    // Extract elements
    for i in (1..n).rev() {
        ops.push(SortOp::Swap(0, i));
        data.swap(0, i);
        heapify(data, i, 0, ops);
    }
}

fn heapify(data: &mut [f64], n: usize, i: usize, ops: &mut Vec<SortOp>) {
    let mut largest = i;
    let left = 2 * i + 1;
    let right = 2 * i + 2;

    if left < n {
        ops.push(SortOp::Compare(left, largest));
        if data[left] > data[largest] {
            largest = left;
        }
    }
    if right < n {
        ops.push(SortOp::Compare(right, largest));
        if data[right] > data[largest] {
            largest = right;
        }
    }
    if largest != i {
        ops.push(SortOp::Swap(i, largest));
        data.swap(i, largest);
        heapify(data, n, largest, ops);
    }
}

impl Animation for Sort {
    fn name(&self) -> &str {
        "sort"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let w = canvas.width;
        let h = canvas.height;

        // Resize data if needed
        let target_size = (w / 2).clamp(16, 200);
        if self.data.len() != target_size {
            self.data = (0..target_size)
                .map(|_| self.rng.random_range(0.05..1.0))
                .collect();
            self.generate_ops();
        }

        // Process operations
        if !self.sorted {
            for _ in 0..self.ops_per_frame {
                if self.op_index < self.ops.len() {
                    match self.ops[self.op_index] {
                        SortOp::Compare(a, b) => {
                            self.active_indices = (a, b);
                        }
                        SortOp::Swap(a, b) => {
                            if a < self.data.len() && b < self.data.len() {
                                self.data.swap(a, b);
                            }
                            self.active_indices = (a, b);
                        }
                    }
                    self.op_index += 1;
                } else {
                    self.sorted = true;
                    self.pause_timer = 2.0;
                    break;
                }
            }
        } else {
            self.pause_timer -= dt;
            if self.pause_timer <= 0.0 {
                self.algo = self.algo.next();
                self.shuffle();
            }
        }

        // Render
        canvas.clear();

        let n = self.data.len();
        let bar_w = (w / n).max(1);

        for i in 0..n {
            let bar_h = (self.data[i] * h as f64) as usize;
            let bar_x = i * bar_w;

            let is_active = i == self.active_indices.0 || i == self.active_indices.1;

            for dy in 0..bar_h {
                let y = h.saturating_sub(1 + dy);
                let frac = dy as f64 / h as f64;

                let (r, g, b) = if is_active {
                    (255, 50, 50) // Highlight active
                } else if self.sorted {
                    // Green when sorted
                    let f = i as f64 / n as f64;
                    (
                        (50.0 + 100.0 * f) as u8,
                        200,
                        (50.0 + 100.0 * (1.0 - f)) as u8,
                    )
                } else {
                    // Normal: color by value
                    let hue = self.data[i] * 0.7;
                    hsv_to_rgb(hue, 0.8, 0.7 + frac * 0.3)
                };

                for bx in 0..bar_w.saturating_sub(if bar_w > 2 { 1 } else { 0 }) {
                    let px = bar_x + bx;
                    if px < canvas.width && y < canvas.height {
                        canvas.set_colored(px, y, 0.7 + frac * 0.3, r, g, b);
                    }
                }
            }
        }

        let _ = self.algo.name();
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
