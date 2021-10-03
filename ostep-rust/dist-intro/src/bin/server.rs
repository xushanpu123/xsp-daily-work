use std::net::UdpSocket;
fn main(){
   let socket = UdpSocket::bind("0.0.0.0:10000").unwrap();
   while true{
       let mut message = [0;1000];
       println!("server:: waiting...");
       let rc = socket.recv_from(&mut message).unwrap().0;
       println!("server:: read message [size:{} contents:{}]\n", rc, std::str::from_utf8(&message).unwrap())
   }
}
/*int main(int argc, char *argv[]) {
    int sd = UDP_Open(10000);
    assert(sd > -1);
    while (1) {
	struct sockaddr_in addr;
	char message[BUFFER_SIZE];
	printf("server:: waiting...\n");
	int rc = UDP_Read(sd, &addr, message, BUFFER_SIZE);
	printf("server:: read message [size:%d contents:(%s)]\n", rc, message);
	if (rc > 0) {
            char reply[BUFFER_SIZE];
            sprintf(reply, "goodbye world");
            rc = UDP_Write(sd, &addr, reply, BUFFER_SIZE);
	    printf("server:: reply\n");
	} 
    }
    return 0; 
}*/
    