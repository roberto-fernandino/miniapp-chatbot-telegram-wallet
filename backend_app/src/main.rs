use anyhow::Result;
use crate::routes::{index, add_or_update_user, get_all_users, get_user};
mod routes;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = tide::new();
    println!("Listening on port 8000");

    app.at("/").get(index);
    app.at("/add_or_update_user").post(add_or_update_user);
    app.at("/all_users").get(get_all_users);
    app.at("/user/:id").get(get_user);
    app.listen("0.0.0.0:8000").await?;

    Ok(())
}

