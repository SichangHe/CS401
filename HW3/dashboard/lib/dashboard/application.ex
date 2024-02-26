defmodule Dashboard.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # Start the Telemetry supervisor
      DashboardWeb.Telemetry,
      # Start the PubSub system
      {Phoenix.PubSub, name: Dashboard.PubSub},
      # Start the Endpoint (http/https)
      DashboardWeb.Endpoint,
      {Dashboard.Redis,
       host: System.get_env("REDIS_HOST", "localhost"),
       port:
         case System.get_env("REDIS_PORT") do
           nil -> 6379
           port -> String.to_integer(port)
         end}
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: Dashboard.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    DashboardWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
