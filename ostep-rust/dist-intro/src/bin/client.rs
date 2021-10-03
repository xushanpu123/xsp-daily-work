use std::net::UdpSocket;
use std::str::from_utf8;
fn main(){
   
   let socket = UdpSocket::bind("0.0.0.0:20000").unwrap();

   let message = "hello world".as_bytes();
   println!("client:: send message [{}]",from_utf8(message).unwrap());
   let rc = socket.send_to(&message, "0.0.0.0:10000").unwrap();
   println!("client:: wait for reply...\n");
   let mut message2 = [0;1000];
   let rc = socket.recv_from(&mut message2).unwrap().0;
   println!("client:: got reply [size:{} contents:({})",rc,from_utf8(message).unwrap());
}
/*int main(int argc, char *argv[]) {
    struct sockaddr_in addrSnd, addrRcv;

    int sd = UDP_Open(20000);
    int rc = UDP_FillSockAddr(&addrSnd, "localhost", 10000);

    char message[BUFFER_SIZE];
    sprintf(message, "hello world");

    printf("client:: send message [%s]\n", message);
    rc = UDP_Write(sd, &addrSnd, message, BUFFER_SIZE);
    if (rc < 0) {
	printf("client:: failed to send\n");
	exit(1);
    }

    printf("client:: wait for reply...\n");
    rc = UDP_Read(sd, &addrRcv, message, BUFFER_SIZE);
    printf("client:: got reply [size:%d contents:(%s)\n", rc, message);
    return 0;
}*/