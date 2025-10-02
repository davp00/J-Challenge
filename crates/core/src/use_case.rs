use async_trait::async_trait;

// Traits
#[async_trait]
pub trait UseCase<In, Out, Err>: Send + Sync
where
    In: Send + 'static,
    Out: Send + 'static,
    Err: Send + 'static,
{
    async fn execute(&self, input: In) -> Result<Out, Err>;
}

#[async_trait]
pub trait UseCaseValidatable<In, Out, Err>: UseCase<In, Out, Err> + Send + Sync
where
    In: Send + Sync + 'static, // Sync porque tomamos &In a través de await
    Out: Send + 'static,
    Err: Send + 'static,
{
    async fn validate(&self, input: &In) -> Result<(), Err>;

    async fn validate_and_execute(&self, input: In) -> Result<Out, Err> {
        self.validate(&input).await?;
        self.execute(input).await
    }
}
