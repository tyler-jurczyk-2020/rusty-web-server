console.log("I was successfully served!");

function call_server() {
    console.log("Calling server")
    fetch("/stats")
        .then(res=>res.text())
        .then(data=>console.log(data));
}

//while(true) {
//    call_server();
//}
