import pytest
from pathlib import Path
from unittest.mock import patch
from whisper_bot.utils import convert_to_wav


def test_convert_to_wav(tmp_path):
    input_file = tmp_path / "input.ogg"
    input_file.touch()

    # We mock ffmpeg so we don't actually need a valid audio file
    with patch("ffmpeg.input") as mock_input:
        mock_output = mock_input.return_value.output.return_value
        mock_run = mock_output.run

        output_file = convert_to_wav(input_file)

        assert output_file.suffix == ".wav"
        # assert output_file.exists()  <-- Removed because mock doesn't create file

        # Check ffmpeg calls
        mock_input.assert_called_with(str(input_file))

        # Check output args
        args, kwargs = mock_input.return_value.output.call_args
        assert str(output_file) == args[0]
        assert kwargs["ar"] == 16000
        assert kwargs["ac"] == 1
