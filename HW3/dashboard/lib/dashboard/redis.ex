defmodule Dashboard.Redis do
  use GenServer
  require Logger

  def start_link(args \\ []) do
    GenServer.start_link(__MODULE__, args, name: __MODULE__)
  end

  @impl true
  def init(_args) do
    initial_state = 0
    send(self(), :poll)
    {:ok, initial_state}
  end

  @impl true
  def handle_info(:poll, state) do
    # <https://hexdocs.pm/phoenix/telemetry.html#telemetry-events>
    :telemetry.execute([:monitoring, :redis], %{poll: state}, %{})
    Logger.info("Polling Redis \##{state}.")
    Process.send_after(self(), :poll, 2_500)
    {:noreply, state + 1}
  end
end
