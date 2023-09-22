use rayon::slice::ParallelSliceMut;
use rustc_data_structures::fx::FxHashMap;
use serde::{Deserialize, Serialize};
use std::io::BufReader;

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
    let file = std::fs::File::open("posts.json").unwrap();
    let reader = BufReader::with_capacity(1024 * 512, file);

    let posts: Vec<Post> = serde_json::from_reader(reader).unwrap();

    let mut post_tags_map: FxHashMap<&String, Vec<&Post>> = FxHashMap::default();

    for post in &posts {
        for tag in &post.tags {
            post_tags_map
                .entry(tag)
                .and_modify(|v| v.push(post))
                .or_insert_with(|| {
                    let mut v = Vec::with_capacity(post.tags.len());
                    v.push(post);
                    v
                });
        }
    }

    let mut related_posts: Vec<RelatedPosts> = Vec::with_capacity(posts.len());
    let mut related_posts_map: FxHashMap<&Post, i8> = FxHashMap::default();

    related_posts_map.reserve(posts.len());

    for post in posts.iter() {
        for tag in &post.tags {
            if let Some(tag_posts) = post_tags_map.get(tag) {
                for other_post in tag_posts {
                    if post._id != other_post._id {
                        *related_posts_map.entry(other_post).or_default() += 1;
                    }
                }
            }
        }

        let mut related_posts_for_post: Vec<_> = Vec::with_capacity(posts.len());
        related_posts_for_post.extend(related_posts_map.drain());

        // related_posts_for_post.sort_unstable_by_key(|&(_, count)| -count);
        related_posts_for_post.par_sort_unstable_by(|post_a, post_b| post_b.1.cmp(&post_a.1));

        related_posts.push(RelatedPosts {
            _id: &post._id,
            tags: &post.tags,
            related: related_posts_for_post
                .into_iter()
                .map(|(post, _)| post)
                .take(5)
                .collect(),
        });
    }

    // Write the result to a JSON file.
    let json = serde_json::to_vec(&related_posts).unwrap();

    std::fs::write("../related_posts.json", json).unwrap();
}
