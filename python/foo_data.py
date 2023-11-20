import requests

response = requests.get("http://127.0.0.1:7878")
print(response.text)
