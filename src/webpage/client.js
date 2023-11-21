console.log("I was successfully served!");

function call_server() {
    fetch("/stats")
        .then(res=>res.text())
        .then(data=>console.log(data));
}

for(let i=0; i<1000; i++) {
    call_server();
}
