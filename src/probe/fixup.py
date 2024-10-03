import pandas as pd
import sys


def main(input_csv: str, meter_id: str, output_csv: str):
  df = pd.read_csv(input_csv)  # type: ignore
  condition = df['MeterId'] == meter_id
  filtered_df = df[condition]
  filtered_df.to_csv(output_csv, index=False)


if __name__ == "__main__":
  if len(sys.argv) != 4:
    print("Usage: python fixup.py <input_csv> <meter_id> <output_csv>")
  else:
    main(sys.argv[1], sys.argv[2], sys.argv[3])
