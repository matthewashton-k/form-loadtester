# Load Tester / Multipart Form Fuzzer

I created this to test out a custom http server and multipart form field parser I am building as a learning experience!

## Features
1. Scraping information from crt.sh to filter publicly available information on subdomains down to active/up subdomains.
2. Custom configuration language and parser for defining the random generation of form feilds.
3. Configurable concurrency level.

## Configuration Language
To fuzz a form you must supply a configuration file that specifies the entries and methods used to create paramaters for any multipart form. <br>
Available commands:

For static parameters
```
static_str(name,val)
```

Randomly generated email with a specified set of domains
```
email(name,domains: arr)
```

Set of key value pairs, of which a random subset is chosen to be included in the multipart form.
```
choose_any(kvps: arr<(k,v)>)
```

Set of key value pairs, of which a random subset of size n is chosen to be included in the multipart form.
```
choose_n(n: usize,kvps: arr<(k,v)>)
```

Randomly generated cellphone
```
cellphone(name)
```

Randomly Generated Date between year min and year max
```
date(name,min,max)
```

Randomly generated string with a maximum length.
```
string(name,maxlen)
```

Randomly generated string that has a first last section separated by whitespace.
```
name(name,maxlen)
```


## Example config file:

```
static("field name","static value")
string("freeresponse",100)
cellphone("usercell")
static("empty static"," ")
choose_n(1,[("radioentry","Yes"),("radioentry","No")])
choose_any([("checkboxes","opt1"),("checkboxes","opt2")])
```

## TODO:
1. Make parse failiure error messages nicer.
2. Allow for whitespace/trailing commas in the config.

<br>
<br>
<br>

*Why write a custom config language and parser you ask?*
*...idk I felt like it.*
