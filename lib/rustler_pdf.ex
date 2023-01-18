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

  def e_create_pdf() do
    Pdf.build([size: :a4, compress: true], fn pdf ->
      pdf
      |> Pdf.set_info(title: "Demo PDF")
      |> Pdf.set_font("Helvetica", 10)
      |> Pdf.text_at({200, 200}, "Spirit")
      |> Pdf.write_to("test.pdf")
    end)
  end

  def modify_pdf_with_configuration() do
    %{
      input_file_path: "PIT-8C.pdf",
      operations: [
        %{
          dimensions: {462.82, 55.92},
          field: :income,
          font: {"F1", 10},
          page_number: 0,
          value: "120.99"
        },
        %{
          dimensions: {43.32, 347.81},
          field: :income,
          font: {"F1", 10},
          page_number: 0,
          value: "41.0"
        }
      ],
      output_file_path: "PIT-8C-modified.pdf"
    } |> RustlerPdf.modify_pdf()
  end

  def benchmark() do
    Benchee.run(
      %{
        "create_pdf" => fn -> RustlerPdf.create_pdf(RustlerPdf.read_config()) end,
        "e_create_pdf" => fn -> RustlerPdf.e_create_pdf() end
      },
      time: 10,
      memory_time: 1
    )
  end

  # When loading a NIF module, dummy clauses for all NIF function are required.
  # NIF dummies usually just error out when called when the NIF is not loaded, as that should never normally happen.
  def add(_arg1, _arg2), do: :erlang.nif_error(:nif_not_loaded)
  def modify_pdf(_arg), do: :erlang.nif_error(:nif_not_loaded)
  def read_config(), do: :erlang.nif_error(:nif_not_loaded)
  def create_pdf(_pdf_writer_configuration), do: :erlang.nif_error(:nif_not_loaded)
end
