console.log("I was successfully served!");

function call_server() {
    fetch("/stats")
        .then(res=>res.text())
        .then(data=>console.log(data));
}

call_server();
