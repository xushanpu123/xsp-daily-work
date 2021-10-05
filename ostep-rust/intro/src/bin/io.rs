use std::io::{self,Write};
use std::env::args;
use crate::common::Spin;


fn main() {
  let argv = args();
  let argc = argv.len();
  if argc != 2{
  let stderr = io::stderr();
  stderr.write(b"usage: cpu <string>\n");
  }
  else{
      let argv1 = argv.nth(1).unwrap();
      loop{
          println!("{}",argv1);
          Spin(1);
      }
  }
}


/*int main(int argc, char *argv[])
{
    if (argc != 2) {
	fprintf(stderr, "usage: cpu <string>\n");
	exit(1);
    }
    char *str = argv[1];

    while (1) {
	printf("%s\n", str);
	Spin(1);
    }
    return 0;
}*/