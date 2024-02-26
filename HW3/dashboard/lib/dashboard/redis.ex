defmodule Dashboard.Redis do
  use GenServer
  require Logger

  def start_link(args \\ []) do
    {:ok, redis} = Redix.start_link(host: "localhost", port: 6379)
    GenServer.start_link(__MODULE__, redis, [{:name, __MODULE__} | args])
  end

  @impl true
  def init(redis) do
    send(self(), :poll)
    {:ok, redis}
  end

  @impl true
  def handle_info(:poll, redis) do
    {:ok, metrics_bytes} = Redix.command(redis, ["GET", "sh623-proj3-output"])
    Logger.info("Polling Redis returned #{metrics_bytes}.")
    metrics = Jason.decode!(metrics_bytes)
    # <https://hexdocs.pm/phoenix/telemetry.html#telemetry-events>
    :telemetry.execute(
      [:monitoring],
      %{
        percentage_memory_caching: metrics["percentage_memory_caching"],
        percentage_outgoing_bytes: metrics["percentage_outgoing_bytes"]
      }
    )

    0..8192
    |> Enum.take_while(fn cpu_index ->
      cpu_key = "moving_average_cpu_percent-#{cpu_index}"

      case Map.get(metrics, cpu_key) do
        nil ->
          false

        cpu_percent ->
          :telemetry.execute([:monitoring], %{moving_average_cpu_percent: cpu_percent}, %{cpu_index: cpu_index})
          true
      end
    end)

    Process.send_after(self(), :poll, 2_500)
    {:noreply, redis}
  end
end
