import requests
import time

url = "http://127.0.0.1:7878"

response = requests.get(url)
print(response.text)

while(True):
    requests.post(url, data="Here is some data")
    time.sleep(5)
