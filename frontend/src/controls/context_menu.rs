use yew::html;
use yew::prelude::*;
use stdweb::js;

pub struct ContextMenu {
   props: Props,
   link: ComponentLink<Self>,
   pos: Pos2D,
}

#[derive(Clone, PartialEq)]
pub struct Pos2D {
   pub x: i32,
   pub y: i32,
}

pub enum Msg {
}

#[derive(Clone, Properties)]
pub struct Props {
   #[props(required)]
   pub pos: Pos2D,
   #[props(required)]
   pub close_cb: Callback<()>,
   pub children: Children,
}

macro_rules! vchange {
	($ch:expr, $to:expr, $from:expr) => {
		if $to != $from {
			$to = $from;
			$ch = true;
		}
	};
}

impl Component for ContextMenu {
   type Message = Msg;
   type Properties = Props;

   fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
      let close_context = props.close_cb.clone();
      js!{
         closeDropdowns();
         document.global_ctxm = @{move || close_context.emit(())};
      };
      let pos = props.pos.clone();
      let res = Self {
         props,
         link,
         pos,
      };
      res
   }

   fn update(&mut self, msg: Self::Message) -> ShouldRender {
      match msg {
      }
   }

   fn change(&mut self, props: Self::Properties) -> ShouldRender {
      let mut changed = false;
      vchange!(changed, self.pos, props.pos);
      changed
   }

   fn view(&self) -> Html {
      html! {
         <div style={format!("position:fixed; z-index: 1; left:{}px; top:{}px", self.pos.x, self.pos.y)}>
            <div class="dropdown is-active keep-active">
               <div class="dropdown-menu" role="menu">
                  <div class="dropdown-content">
                     {
                        for self.props.children.iter().map(|c| {
                           html! {
                              <div class="dropdown-item">
                                 { c }
                              </div>
                           }
                        })
                     }
                  </div>
               </div>
            </div>
         </div>
      }
   }
}