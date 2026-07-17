import subprocess

result = subprocess.run(
    ["./main"],
    input="Armin",
    text=True,
    capture_output=True
)

print(result.stdout)
