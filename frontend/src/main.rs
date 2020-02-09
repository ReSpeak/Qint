#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	console_log::init_with_level(Level::Debug);
	yew::start_app::<qint_frontend::Model>();
}
