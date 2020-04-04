use std::sync::Arc;

use actix_web::*;
use diesel::prelude::*;
use juniper::{EmptyMutation, EmptySubscription, FieldError, ID, RootNode};
use juniper::http::graphiql::graphiql_source;
use juniper::http::GraphQLRequest;

use crate::State;
use super::{models, schema, RunOnDbMsg};

const BOOKMARKS_LIMIT: i64 = 20;

#[derive(Clone)]
pub struct Context;
#[derive(Clone)]
pub struct Query;

struct Bookmark(models::Bookmark);

pub(crate) type Schema = RootNode<'static, Query, EmptyMutation<State>, EmptySubscription<State>>;
type GResult<T> = std::result::Result<T, FieldError>;

#[get("/graphiql")]
pub async fn graphiql() -> impl Responder {
	HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(graphiql_source("/db"))
}

#[post("/db")]
pub(crate) async fn db_graphql(state: web::Data<State>, data: web::Json<GraphQLRequest>) -> Result<impl Responder> {
	let res = data.execute(&state.graphql_schema, &*state).await;
	let res = serde_json::to_string(&res)?;
	Ok(HttpResponse::Ok()
		.content_type("application/json")
		.body(res))
}

#[juniper::graphql_object]
/// A previously visited server.
impl Bookmark {
	/// The internal id of the bookmark.
	fn id(&self) -> ID { ID::new(self.0.id.to_string()) }
	/// The name of the bookmark if it has a custom name.
	fn name(&self) -> Option<&str> { self.0.name.as_ref().map(|s| s.as_str()) }
	/// The name that was used to connect.
	fn username(&self) -> &str { &self.0.username }
	/// The server address.
	fn address(&self) -> &str { &self.0.address }
}

#[juniper::graphql_object(Context = State)]
impl Query {
	// TODO Support pagination: https://relay.dev/graphql/connections.htm
	async fn bookmarks(state: &State) -> GResult<Vec<Bookmark>> {
		let res = state.database.send(RunOnDbMsg(|db| {
			use diesel::dsl::not;
			use schema::{bookmarks, channels, servers};

			// Order by (bookmark, last_used)
			// Select id, name, address, bookmark, last_used, timezone
			// Join channel.name
			// Join server.icon

			let query = bookmarks::table
				.left_outer_join(servers::table)
				.left_outer_join(
					channels::table.on(bookmarks::server
						.eq(channels::server.nullable())
						.and(bookmarks::channel.eq(channels::id.nullable()))),
				)
				.order((bookmarks::bookmark, bookmarks::last_used))
				.limit(BOOKMARKS_LIMIT)
				.select((
					bookmarks::id,
					bookmarks::name,
					bookmarks::username,
					bookmarks::address,
					bookmarks::bookmark,
					bookmarks::last_used,
					bookmarks::timezone,
					channels::name.nullable(),
					servers::icon.nullable(),
				));
			let result = /*if let Some((book, last)) = msg.start {
				// (bookmark == book AND last_used > last) OR (!bookmark AND book)
				query
					.filter(
						bookmarks::bookmark
							.eq(book)
							.and(bookmarks::last_used.gt(Some(last.naive_utc()))),
					)
					.or_filter(not(bookmarks::bookmark).and(book))
					.load::<Bookmark>(&db.con)
			} else*/ {
				query.load::<models::Bookmark>(&db.con)
			}?.into_iter().map(Bookmark).collect();

			GResult::Ok(result)
		})).await??;
		Ok(res)
	}
}

pub(crate) fn create_schema() -> Arc<Schema> {
	Arc::new(Schema::new(Query, EmptyMutation::<State>::new(), EmptySubscription::<State>::new()))
}
