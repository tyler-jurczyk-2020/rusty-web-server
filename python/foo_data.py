import requests
import time
import numpy as np

url = "http://127.0.0.1:7878"

session = requests.Session()

response = session.get(url)
print(response.text)

while(True):
    data = np.random.normal(0, 1);
    session.post(url, data=f'{data}')
    time.sleep(5)
