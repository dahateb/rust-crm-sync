$DIR=$PSScriptRoot + "/db"
docker run -p 5432:5432 -v ${DIR}:/docker-entrypoint-initdb.d postgres