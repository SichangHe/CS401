defmodule DashboardWeb.PageController do
  use DashboardWeb, :controller

  def index(conn, _params) do
    redirect(conn, to: "/dashboard/metrics")
  end
end
