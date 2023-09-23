use std::{cmp::Reverse, collections::BinaryHeap};

use rayon::prelude::*;
use rustc_data_structures::fx::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::from_str;

use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
struct Post {
    _id: String,
    title: String,
    // #[serde(skip_serializing)]
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RelatedPosts<'a> {
    _id: &'a String,
    tags: &'a Vec<String>,
    related: Vec<&'a Post>,
}

fn main() {
    let json_str = std::fs::read_to_string("../posts.json").unwrap();
    let posts: Vec<Post> = from_str(&json_str).unwrap();

    let mut post_tags_map: FxHashMap<&String, Vec<usize>> = FxHashMap::default();

    for (i, post) in posts.iter().enumerate() {
        for tag in &post.tags {
            post_tags_map.entry(tag).or_default().push(i);
        }
    }

    let (sender, receiver) = crossbeam::channel::bounded(1000);
    let (done_sender, done_receiver) = crossbeam::channel::bounded(1);

    std::thread::scope(|s| {
        s.spawn(|| {
            let mut related_posts: Vec<RelatedPosts> = Vec::with_capacity(posts.len());

            while let Ok(related_post) = receiver.recv() {
                match related_post {
                    Some(related_post) => related_posts.push(related_post),
                    None => {
                        done_sender.send(related_posts).unwrap();
                        break;
                    }
                }
            }
        });

        posts.par_iter().enumerate().for_each(|(idx, post)| {
            let mut tagged_post_count = vec![0; posts.len()];
            tagged_post_count.fill(0);

            for tag in &post.tags {
                if let Some(tag_posts) = post_tags_map.get(tag) {
                    for other_post_idx in tag_posts {
                        if idx != *other_post_idx {
                            tagged_post_count[*other_post_idx] += 1;
                        }
                    }
                }
            }

            let mut top_five = BinaryHeap::new();
            tagged_post_count
                .iter()
                .enumerate()
                .for_each(|(post, count)| {
                    if top_five.len() < 5 {
                        top_five.push((Reverse(count), post));
                    } else {
                        let (Reverse(cnt), _) = top_five.peek().unwrap();
                        if count > *cnt {
                            top_five.pop();
                            top_five.push((Reverse(count), post));
                        }
                    }
                });

            sender
                .send(Some(RelatedPosts {
                    _id: &post._id,
                    tags: &post.tags,
                    related: top_five.into_iter().map(|(_, post)| &posts[post]).collect(),
                }))
                .unwrap();
        });

        sender.send(None).unwrap();

        let related_posts = done_receiver.recv().unwrap();
        let json_str = serde_json::to_string(&related_posts).unwrap();
        std::fs::write("../related_posts_rust_rayon.json", json_str).unwrap();
    });
}
