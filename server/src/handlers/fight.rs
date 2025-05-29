use std::time::Duration;
use rocket::{futures::*, State};
use sea_orm::{prelude::*, *};
use ebobo_shared::Utc;

use crate::{
    entities::{fighters, matches, plays, prelude::*},
    EboboState,
};

#[options("/fight")]
pub async fn options() {}

#[get("/fight")]
pub async fn post(
    auth: crate::auth::Auth,
    ws: rocket_ws::WebSocket,
    state: &State<EboboState>,
) -> rocket_ws::Channel<'static> {
    // Pure function to update queued status for a fighter
    let update_queue = |db: &DatabaseConnection, fingerprint: &str| async move {
        Fighters::update(fighters::ActiveModel {
            queued: ActiveValue::set(true),
            ..Default::default()
        })
        .filter(fighters::Column::Fingerprint.eq(fingerprint.to_string()))
        .exec(db)
        .await
    };

    // Pure function to find an opponent in the queue
    let find_opponent = |db: &DatabaseConnection, fingerprint: &str| async move {
        Fighters::find()
            .filter(
                fighters::Column::Fingerprint
                    .ne(fingerprint.to_string())
                    .and(fighters::Column::Queued.eq(true)),
            )
            .limit(1)
            .all(db)
            .await
    };

    // Pure function to calculate match results
    let match_outcome = |enemy_rank: i32, my_rank: i32, enemy_fp: String, my_fp: String| {
        let (mut enemy_r, mut my_r) = (enemy_rank + 1, my_rank + 1);
        let winner = if enemy_rank == my_rank {
            enemy_r += 1;
            my_r += 1;
            None
        } else if enemy_rank < my_rank {
            my_r += 2;
            Some(my_fp)
        } else {
            enemy_r += 2;
            Some(enemy_fp)
        };
        (enemy_r, my_r, winner)
    };

    // Pure function to update ranks and queue status
    let update_fighter = |db: &DatabaseConnection, fingerprint: &str, rank: i32| async move {
        Fighters::update(fighters::ActiveModel {
            rank: ActiveValue::set(rank),
            queued: ActiveValue::set(false),
            ..Default::default()
        })
        .filter(fighters::Column::Fingerprint.eq(fingerprint.to_string()))
        .exec(db)
        .await
    };

    // Pure function to create match and plays
    let create_match_and_plays = |db: &DatabaseConnection, winner: Option<String>, my_fp: String, enemy_fp: String| async move {
        let id = Uuid::new_v4();
        Matches::insert(matches::ActiveModel {
            id: ActiveValue::set(id),
            winner: ActiveValue::set(winner.clone()),
            date: ActiveValue::set(Utc::now().naive_utc()),
        });
        let play_futures = vec![
            Plays::insert(plays::ActiveModel {
                r#match: ActiveValue::set(id),
                fighter: ActiveValue::set(my_fp.clone()),
                ..Default::default()
            })
            .exec(db),
            Plays::insert(plays::ActiveModel {
                r#match: ActiveValue::set(id),
                fighter: ActiveValue::set(enemy_fp.clone()),
                ..Default::default()
            })
            .exec(db),
        ];
        futures::future::join_all(play_futures).await;
        winner
    };

    let db = state.db.clone();
    let fingerprint = auth.fingerprint.clone();

    // All side-effects happen in the async closure for the websocket channel
    ws.channel(move |mut stream| {
        let db = db.clone();
        let fingerprint = fingerprint.clone();
        let auth = auth.clone();
        Box::pin(async move {
            update_queue(&db, &fingerprint).await.unwrap();

            let try_match = || async {
                let queue_result = find_opponent(&db, &fingerprint).await;
                queue_result.ok().and_then(|queue| queue.first().cloned())
            };

            while let Some(enemy) = try_match().await {
                let fighter = auth.fighter.clone().unwrap();

                let (enemy_r, my_r, winner) = match_outcome(enemy.rank, fighter.rank, enemy.fingerprint.clone(), fingerprint.clone());

                let (update_my, update_enemy) = futures::join!(
                    update_fighter(&db, &fingerprint, my_r),
                    update_fighter(&db, &enemy.fingerprint, enemy_r)
                );
                update_my.unwrap();
                update_enemy.unwrap();

                let winner_fp = create_match_and_plays(&db, winner.clone(), fingerprint.clone(), enemy.fingerprint.clone()).await;

                if let Some(winner_fp) = winner_fp {
                    let _ = stream.send(rocket_ws::Message::Text(winner_fp)).await;
                }

                break;
            }

            // Sleep at the end of each loop if no match was found
            tokio::time::sleep(Duration::from_secs(5)).await;

            Ok(())
        })
    })
}
