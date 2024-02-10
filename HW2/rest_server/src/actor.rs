use super::*;

#[derive(Debug)]
pub struct Ref<A: Actor> {
    pub msg_sender: Sender<Msg<A>>,
    pub cancellation_token: CancellationToken,
}

impl<A: Actor> Ref<A> {
    pub fn cast(
        &mut self,
        msg: A::CastMsg,
    ) -> impl Future<Output = Result<(), SendError<Msg<A>>>> + '_ {
        self.msg_sender.send(Msg::Cast(msg))
    }

    pub async fn call(&mut self, msg: A::CallMsg) -> Result<A::Reply> {
        let (reply_sender, reply_receiver) = oneshot::channel();
        self.msg_sender
            .send(Msg::Call(msg, reply_sender))
            .await
            .context("Failed to send call to actor")?;
        reply_receiver
            .await
            .context("Failed to receive actor's reply")
    }

    pub fn cancel(&mut self) {
        self.cancellation_token.cancel()
    }
}

impl<A: Actor> Clone for Ref<A> {
    fn clone(&self) -> Self {
        Self {
            msg_sender: self.msg_sender.clone(),
            cancellation_token: self.cancellation_token.clone(),
        }
    }
}

pub enum Msg<A: Actor> {
    Call(A::CallMsg, oneshot::Sender<A::Reply>),
    Cast(A::CastMsg),
}

pub trait Actor: Sized + Send + 'static {
    type CallMsg: Send + Sync;
    type CastMsg: Send + Sync;
    type Reply: Send;

    fn handle_cast(
        &mut self,
        msg: Self::CastMsg,
        env: &Ref<Self>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn handle_call(
        &mut self,
        msg: Self::CallMsg,
        env: &Ref<Self>,
        reply_sender: oneshot::Sender<Self::Reply>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn handle_call_or_cast(
        &mut self,
        msg: Msg<Self>,
        env: &Ref<Self>,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            match msg {
                Msg::Call(msg, reply_sender) => self.handle_call(msg, env, reply_sender).await,
                Msg::Cast(msg) => self.handle_cast(msg, env).await,
            }
        }
    }

    fn handle_continuously(
        &mut self,
        mut receiver: Receiver<Msg<Self>>,
        env: Ref<Self>,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            loop {
                let maybe_msg = select! {
                    m = receiver.recv() => m,
                    () = env.cancellation_token.cancelled() => return Ok(()),
                };

                let msg = match maybe_msg {
                    Some(m) => m,
                    None => return Ok(()),
                };

                select! {
                    maybe_ok = self.handle_call_or_cast(msg, &env) => maybe_ok,
                    () = env.cancellation_token.cancelled() => return Ok(()),
                }?;
            }
        }
    }

    fn spawn(self) -> (JoinHandle<Result<()>>, Ref<Self>) {
        let (msg_sender, msg_receiver) = channel(8);
        let cancellation_token = CancellationToken::new();
        self.spawn_with_channel_and_token(msg_sender, msg_receiver, cancellation_token)
    }

    fn spawn_with_channel_and_token(
        mut self,
        msg_sender: Sender<Msg<Self>>,
        msg_receiver: Receiver<Msg<Self>>,
        cancellation_token: CancellationToken,
    ) -> (JoinHandle<Result<()>>, Ref<Self>) {
        let actor_ref = Ref {
            msg_sender,
            cancellation_token,
        };
        let handle = {
            let env = actor_ref.clone();
            spawn(async move { self.handle_continuously(msg_receiver, env).await })
        };

        (handle, actor_ref)
    }
}
