DIR=$(pwd)
echo $DIR
sudo docker run -p 5432:5432 -v $DIR/db:/docker-entrypoint-initdb.d postgres:9.6
