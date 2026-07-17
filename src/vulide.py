import subprocess

result = subprocess.run(
    ["./main"],
    input="BatScript",
    text=True,
    capture_output=True
)

print(result.stdout)
