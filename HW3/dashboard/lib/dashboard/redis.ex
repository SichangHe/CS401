defmodule Dashboard.Redis do
  use GenServer
  require Logger

  def start_link(args \\ []) do
    host = Keyword.get(args, :host)
    port = Keyword.get(args, :port)
    output_key = Keyword.get(args, :output_key)
    {:ok, redis} = Redix.start_link(host: host, port: port)

    poll_fn = fn -> Redix.command(redis, ["GET", output_key]) end

    GenServer.start_link(__MODULE__, poll_fn, [{:name, __MODULE__} | args])
  end

  @impl true
  def init(poll_fn) do
    send(self(), :poll)
    {:ok, poll_fn}
  end

  @impl true
  def handle_info(:poll, poll_fn) do
    {:ok, metrics_bytes} = poll_fn.()
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
          :telemetry.execute([:monitoring], %{moving_average_cpu_percent: cpu_percent}, %{
            cpu_index: cpu_index
          })

          true
      end
    end)

    Process.send_after(self(), :poll, 2_500)
    {:noreply, poll_fn}
  end
end
