use rand::{rngs::ThreadRng, Rng};

#[derive(Debug)]
pub struct Guitar {
    strings: Vec<Vec<f32>>,
    indices: Vec<usize>,
    time: i64,
}

impl Guitar {
    pub fn new(sample_rate: u32) -> Self {
        let mut strings: Vec<Vec<f32>> = Vec::new();
        for i in 0..37 {
            let freq = 440f64 * 2.0f64.powf((i as f64 - 24.0) / 12.0);
            let mut tmp = Vec::new();
            tmp.resize((sample_rate as f64 / freq) as usize, 0f32);
            strings.push(tmp);
        }
        Self {
            strings,
            indices: Vec::from([0usize; 37]),
            time: 0,
        }
    }

    fn noise(&mut self, id: usize, rng: &mut ThreadRng) {
        let v = &mut self.strings[id];
        for i in 0..v.len() {
            v[i] = rng.random_range(0..65536) as f32 / 65536.0 - 0.5;
            debug_assert!(v[i] >= -0.5 && v[i] < 0.5);
        }
    }

    pub fn pluck(&mut self, str: isize, rng: &mut ThreadRng) {
        let id = str + 24;
        if id >= 0 && id < 37 {
            self.noise(id as usize, rng);
        }
    }

    pub fn tick(&mut self) -> f32 {
        let mut sample = 0f32;
        for i in 0..37usize {
            let v = &mut self.strings[i];
            let ptr = self.indices[i];
            let size = v.len();
            sample += v[ptr];
            v[ptr] = (0.996 / 2.0) * (v[ptr] + v[(ptr + 1) % size]);
            self.indices[i] = (ptr + 1) % size;
        }
        self.time += 1;
        sample
    }

    pub fn time(&self) -> i64 {
        self.time
    }
}
