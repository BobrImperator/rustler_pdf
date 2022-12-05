defmodule RustlerPdf do
  use Rustler,
    otp_app: :rustler_pdf,
    crate: :rustlerpdf

  @moduledoc """
  Documentation for `RustlerPdf`.
  """

  @doc """
  Hello world.

  ## Examples

      iex> RustlerPdf.hello()
      :world

  """
  def hello do
    :world
  end

  # When loading a NIF module, dummy clauses for all NIF function are required.
  # NIF dummies usually just error out when called when the NIF is not loaded, as that should never normally happen.
  def add(_arg1, _arg2), do: :erlang.nif_error(:nif_not_loaded)
  def r_modify_pdf(_arg), do: :erlang.nif_error(:nif_not_loaded)
  def r_read_config(), do: :erlang.nif_error(:nif_not_loaded)
end
