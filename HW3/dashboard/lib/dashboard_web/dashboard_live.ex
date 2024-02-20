defmodule DashboardWeb.Live do
  use Phoenix.LiveDashboard.PageBuilder
  import Phoenix.LiveDashboard.Helpers

  @impl true
  def menu_link(_, _) do
    {:ok, "Monitoring"}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <.live_table
      id="ets-table"
      dom_id="ets-table"
      page={@page}
      title="Monitoring"
      row_fetcher={&fetch_ets/2}
      row_attrs={&row_attrs/1}
      rows_name="tables"
    >
      <:col field={:name} header="Name or module" />
      <:col field={:protection} />
      <:col field={:type} />
      <:col field={:size} text_align="right" sortable={:desc} />
      <:col :let={ets} field={:memory} text_align="right" sortable={:desc}>
        <%= format_words(ets[:memory]) %>
      </:col>
      <:col :let={ets} field={:owner}>
        <%= encode_pid(ets[:owner]) %>
      </:col>
    </.live_table>
    """
  end

  defp fetch_ets(params, node) do
    %{search: search, sort_by: sort_by, sort_dir: sort_dir, limit: limit} = params

    # TODO: customize
    # Here goes the code that goes through all ETS tables, searches
    # (if not nil), sorts, and limits them.
    #
    # It must return a tuple where the first element is list with
    # the current entries (up to limit) and an integer with the
    # total amount of entries.
    # ...
    Phoenix.LiveDashboard.SystemInfo.fetch_ets(node, search, sort_by, sort_dir, limit)
  end

  defp row_attrs(table) do
    [
      {"phx-click", "show_info"},
      {"phx-value-info", "placeholder"},
      {"phx-page-loading", true}
    ]
  end
end
