import requests
import time
import numpy as np

url = "http://127.0.0.1:7878"

response = requests.get(url)
print(response.text)

while(True):
    data = np.random.normal(0, 1);
    requests.post(url, data=f'{data}')
    time.sleep(5)
