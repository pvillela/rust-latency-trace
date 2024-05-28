use rand::Rng;

pub fn gen_random_to(max: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0..=max)
}
