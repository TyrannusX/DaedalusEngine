use application::run;

mod application;

fn main() {
    pollster::block_on(run())
}
