use DATACOM::run;

fn main() {
    pollster::block_on(run());
}