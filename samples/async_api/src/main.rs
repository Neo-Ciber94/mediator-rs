#![allow(dead_code)]
use mediator::{AsyncMediator, AsyncRequestHandler, DefaultAsyncMediator, Event, Request};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct User {
    id: Uuid,
    name: String,
}

struct UserService(Vec<User>);
type SharedUserService = Arc<Mutex<UserService>>;

struct CreateUserRequest(String);
impl Request<User> for CreateUserRequest {}

struct GetAllUsersRequest;
impl Request<Vec<User>> for GetAllUsersRequest {}

#[derive(Clone)]
struct UserCreatedEvent(User);
impl Event for UserCreatedEvent {}

struct CreateUserRequestHandler(SharedUserService, DefaultAsyncMediator);
#[mediator::async_trait]
impl AsyncRequestHandler<CreateUserRequest, User> for CreateUserRequestHandler {
    async fn handle(&mut self, req: CreateUserRequest) -> User {
        let mut service = self.0.lock().await;
        let user = User {
            id: Uuid::new_v4(),
            name: req.0.clone(),
        };

        service.0.push(user.clone());
        self.1
            .publish(UserCreatedEvent(user.clone()))
            .await
            .expect("publish failed");
        user
    }
}

#[tokio::main]
async fn main() {
    let service = Arc::new(Mutex::new(UserService(vec![])));
    let total_users = Arc::new(Mutex::new(0_usize));

    let mut mediator = DefaultAsyncMediator::builder()
        .add_handler_deferred(|m| CreateUserRequestHandler(service.clone(), m))
        .add_handler_fn_with(
            service.clone(),
            move |_: GetAllUsersRequest, service| async move {
                let lock = service.lock().await;
                let users = lock.0.clone();
                users
            },
        )
        .subscribe_fn_with(
            total_users.clone(),
            |event: UserCreatedEvent, total: Arc<Mutex<usize>>| async move {
                println!("User created: {:?}", event.0.name);
                let mut count = total.lock().await;
                *count += 1;
            },
        )
        .build();

    mediator
        .send(CreateUserRequest("John".to_string()))
        .await
        .unwrap();
    mediator
        .send(CreateUserRequest("Jane".to_string()))
        .await
        .unwrap();

    let users = mediator.send(GetAllUsersRequest).await.unwrap();
    println!("{:#?}", users);
    println!("Total users: {}", total_users.lock().await);
}
