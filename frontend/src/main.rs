#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
	web_logger::init();
	yew::start_app::<qint_frontend::Model>();
}
