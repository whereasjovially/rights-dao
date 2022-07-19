
use std::collections::VecDeque;

use candid::Principal;
use ic_cdk_macros::{update, query};

use crate::reputation::domain::{ReputationAction, ReputationEvent};
use crate::CONTEXT;
use crate::common::guard::has_user_guard;

use super::{domain::*, error::PostError};

#[update(guard = "has_user_guard")] 
fn create_post(cmd: PostCreateCommand) -> Result<u64, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let id = ctx.id;
        let caller = ctx.env.caller();
        let now = ctx.env.now();
        let post = cmd.build_profile(
            id,
            caller,
            PostStatus::Enable,
            now
        );
        match ctx.post_service.create_post(post) {
            Some(_) => {
                ctx.id += 1;    // id addOne

                let re = ReputationEvent::new(ctx.id, caller, ReputationAction::PublishPost, 2, now);
                ctx.reputation_service.handle_reputation_event(re);
                ctx.id += 1;

                Ok(id)
            },
            None => Err(PostError::PostAlreadyExists),
        }
    })
}

#[update]
fn edit_post(cmd: PostEditCommand) -> Result<bool, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let caller = ctx.env.caller();
        let post_id = cmd.id;
        let now = ctx.env.now();
        match ctx.post_service.get_post(post_id) {
            Some(p) => {
                if p.author != caller {
                    return Err(PostError::PostUnAuthorizedOperation);
                }
                if p.status == PostStatus::Completed {
                    return Err(PostError::PostAlreadyCompleted);
                }
                ctx.post_service.edit_post(cmd, now).ok_or(PostError::PostNotFound)
            },
            None => Err(PostError::PostNotFound),
        }
        
    })
}

#[update]
fn change_post_status(cmd: PostChangeStatusCommand) -> Result<bool, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let caller = ctx.env.caller();
        let post_id = cmd.id;
        let now = ctx.env.now();
        match ctx.post_service.get_post(post_id) {
            Some(p) => {
                if p.author != caller {
                    return Err(PostError::PostUnAuthorizedOperation);
                }
                
                ctx.post_service.update_post_status(cmd, caller, now)
            },
            None => Err(PostError::PostNotFound),
        }
    })
}

#[update]
fn submit_post_answer(cmd: PostAnswerCommand) -> Result<bool, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let caller = ctx.env.caller();
        let post_id = cmd.post_id;
        let now = ctx.env.now();
        match ctx.post_service.get_post(post_id) {
            Some(p) => {
                if p.author != caller {
                    return Err(PostError::PostUnAuthorizedOperation);
                }
                
                let res = ctx.post_service.update_post_answer(cmd.clone(), now);

                if res.is_ok() {
                    if let Some(comment) = p.comments.iter().find(|c| c.id == cmd.comment_id) {
                        if comment.author != p.author {
                            let re = ReputationEvent::new(ctx.id, comment.author, ReputationAction::SelectedPostAnswer, 1, now);
                            ctx.reputation_service.handle_reputation_event(re);
                        }                   
                    }
                }

                res
            },
            None => Err(PostError::PostNotFound),
        }
    })
}

#[update]
fn delete_post(cmd: PostIdCommand) -> Result<bool, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let caller = ctx.env.caller();
        let post_id = cmd.id;
        match ctx.post_service.get_post(post_id) {
            Some(p) => {               
                if p.author != caller {
                    return Err(PostError::PostUnAuthorizedOperation);
                }
                if p.status == PostStatus::Completed {
                    return Err(PostError::PostAlreadyCompleted);
                }
                ctx.post_service.delete_post(post_id).ok_or(PostError::PostNotFound)
            },
            None => Err(PostError::PostNotFound),
        }
    })
}

#[update]
fn add_post_comment(cmd: PostCommentCommand) -> Result<bool, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let comment_id = ctx.id;
        let caller = ctx.env.caller();
        let now = ctx.env.now();
        let post_id = cmd.post_id;
        
        match ctx.post_service.add_post_comment(cmd, comment_id, caller, now) {
            Ok(_) => {
                ctx.id += 1;    // id addOne
                
                let post = ctx.post_service.get_post(post_id).ok_or(PostError::PostNotFound)?;
                // 回帖人与发帖人不是同一人才能获得被回帖的声望积分
                if post.author != caller {
                    let re = ReputationEvent::new(ctx.id, post.author, ReputationAction::PassiveReplied, 1, now);
                    ctx.reputation_service.handle_reputation_event(re);
                    ctx.id += 1;
                }
                // 回帖人获取声望积分
                let re = ReputationEvent::new(ctx.id, caller, ReputationAction::ReplyPost, 1, now);
                ctx.reputation_service.handle_reputation_event(re);
                ctx.id += 1;

                Ok(true)
            },
            e => e,
        }
    })
}

#[update]
fn add_comment_comment(cmd: CommentCommentCommand) -> Result<bool, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let comment_id = ctx.id;
        let caller = ctx.env.caller();
        let now = ctx.env.now();

        match ctx.post_service.add_comment_comment(cmd, comment_id, caller, now) {
            Ok(_) => {
                ctx.id += 1;    // id addOne
                
                Ok(true)
            },
            e => e,
        }
    })
}

#[update]
fn add_post_event(cmd: PostEventCommand) -> Result<bool, PostError> {
    CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        let caller = ctx.env.caller();
        let now = ctx.env.now();
        ctx.post_service.add_post_event(cmd, caller, now)
    })
}

#[query]
fn get_post(cmd: PostIdCommand) -> Result<PostProfile, PostError> {
    CONTEXT.with(|c| {
        c.borrow().post_service.get_post(cmd.id).ok_or(PostError::PostNotFound)
    })
}

#[query]
fn get_post_info(cmd: PostIdCommand) -> Result<PostInfo, PostError> {
    CONTEXT.with(|c| {
        c.borrow().post_service.get_post(cmd.id).map(|p| p.into()).ok_or(PostError::PostNotFound)
    })
}

#[query]
fn get_post_comments(cmd: PostIdCommand) -> Result<Vec<PostComment>, PostError> {
    CONTEXT.with(|c| {
        c.borrow().post_service.get_post(cmd.id).map(|p| p.comments).ok_or(PostError::PostNotFound)
    })
}

#[query]
fn get_post_events(cmd: PostIdCommand) -> Result<VecDeque<PostEvent>, PostError> {
    CONTEXT.with(|c| {
        c.borrow().post_service.get_post(cmd.id).map(|p| p.events).ok_or(PostError::PostNotFound)
    })
}

#[query]
fn page_posts(query: PostPageQuery) -> Result<PostPage, PostError> {
    CONTEXT.with(|c| {
        Ok(c.borrow().post_service.page_posts(&query))
    })
}

#[query]
fn my_posts(query: PostPageQuery) -> Result<PostInfoPage, PostError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let caller = ctx.env.caller();
        Ok(ctx.post_service.my_posts(caller, &query))
    })
}

#[query]
fn my_post_comments(query: PostPageQuery) -> Result<PostPage, PostError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let caller = ctx.env.caller();
        Ok(ctx.post_service.my_post_comments(caller, &query))
    })
}

#[query]
fn my_comments(query: PostPageQuery) -> Result<CommentSummaryPage, PostError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let caller = ctx.env.caller();
        Ok(ctx.post_service.my_comments(caller, &query))
    })
}

#[query]
fn other_posts(query: PostPageOtherQuery) -> Result<PostInfoPage, PostError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let q = PostPageQuery {
            page_size: query.page_size,
            page_num: query.page_num,
            querystring: query.querystring,
        };

        match Principal::from_text(query.other) {
            Ok(p) => Ok(ctx.post_service.my_posts(p, &q)),
            Err(_) => Err(PostError::UserNotFound)
        }
        
    })
}

#[query]
fn other_post_comments(query: PostPageOtherQuery) -> Result<PostPage, PostError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let q = PostPageQuery {
            page_size: query.page_size,
            page_num: query.page_num,
            querystring: query.querystring,
        };

        match Principal::from_text(query.other) {
            Ok(p) => Ok(ctx.post_service.my_post_comments(p, &q)),
            Err(_) => Err(PostError::UserNotFound)
        }
        
    })
}

#[query]
fn other_comments(query: PostPageOtherQuery) -> Result<CommentSummaryPage, PostError> {
    CONTEXT.with(|c| {
        let ctx = c.borrow();
        let q = PostPageQuery {
            page_size: query.page_size,
            page_num: query.page_num,
            querystring: query.querystring,
        };

        match Principal::from_text(query.other) {
            Ok(p) => Ok(ctx.post_service.my_comments(p, &q)),
            Err(_) => Err(PostError::UserNotFound)
        }
        
    })
}
