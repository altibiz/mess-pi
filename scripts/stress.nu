#!/usr/bin/env nu

def main [] { }

let working_dir = pwd
let root =  $env.FILE_PWD | path dirname
let assets = [ $root "assets" ] | path join
let ozds = [ $assets "ozds.sql" ] | path join
let config = [ $assets "config.toml" ] | path join
let meter_types = open --raw $config
  | from toml
  | get modbus.devices
  | columns

let test_dir = [ $root "test" "stress" ] | path join
let orig_test_file = [ $test_dir "project.gns3" ] | path join
let project_id = open $orig_test_file | from json | get project_id
let meter_switch_ids = open $orig_test_file |
  from json |
  get topology.nodes |
  filter { |x| $x.name | str contains "meter-switch" } |
  get node_id

let test_working_dir = mktemp -d
print $"Working directory: '($test_working_dir)'"
let test_file = [ $test_working_dir "project.gns3" ] | path join
let project_files_dir = [ $test_working_dir "project-files" ] | path join
let vpcs = [ $project_files_dir "vpcs" ] | path join
let docker = [ $project_files_dir "docker" ] | path join

let test_zip = [ (mktemp -d) "project.gns3project" ] | path join

let docker_node_template = {
  "compute_id": "local",
  "console": "{{generated}}",
  "console_auto_start": false,
  "console_type": "telnet",
  "custom_adapters": [],
  "first_port_name": null,
  "height": 60,
  "label": {
    "rotation": 0,
    "style": "font-family: TypeWriter;font-size: 10.0;font-weight: bold;fill: #000000;fill-opacity: 1.0;",
    "text": "{{generated}}",
    "x": 0,
    "y": -30
  },
  "locked": false,
  "name": "{{generated}}",
  "node_id": "{{generated}}",
  "node_type": "docker",
  "port_name_format": "Ethernet{0}",
  "port_segment_size": 0,
  "properties": {
    "adapters": 1,
    "aux": "{{generated}}",
    "console_http_path": "/",
    "console_http_port": 80,
    "console_resolution": "1024x768",
    "container_id": "{{generated}}",
    "environment": "",
    "extra_hosts": null,
    "extra_volumes": [],
    "image": "{{generated}}",
    "start_command": "{{generated}}",
    "usage": ""
  },
  "symbol": ":/symbols/affinity/circle/blue/health.svg",
  "width": 60,
  "x": "{{generated}}",
  "y": "{{generated}}",
  "z": 1
}

let link_template = {
  "filters": {},
  "link_id": "{{generated}}",
  "link_style": {},
  "nodes": [
    {
      "adapter_number": 0,
      "label": {
        "rotation": 0,
        "style": "font-family: TypeWriter;font-size: 10.0;font-weight: bold;fill: #000000;fill-opacity: 1.0;",
        "text": "{{generated}}",
        "x": 26,
        "y": 54
      },
      "node_id": "{{generated}}",
      "port_number": "{{generated}}"
    },
    {
      "adapter_number": 0,
      "label": {
        "rotation": 0,
        "style": "font-family: TypeWriter;font-size: 10.0;font-weight: bold;fill: #000000;fill-opacity: 1.0;",
        "text": "{{generated}}",
        "x": 41,
        "y": -9
      },
      "node_id": "{{generated}}",
      "port_number": "{{generated}}"
    }
  ],
  "suspend": false
}

let interfaces_template = "auto eth0
iface eth0 inet static
  address {{generated}}
  netmask 255.255.255.0
  gateway 10.0.0.1"

def gen_container_id [
  --chars: string = "abcdef0123456789"
  --len: int = 64
] {
  let arr = $chars | split row ""
  generate "" { |$prev| 
    let next = $prev + ($arr | get (random int ..(($arr | length) - 1)) | to text)
    { next: $next out: $next }
  } | first $len | get ($len - 1)
}

def "main images" [] {
  sudo docker pull frrouting/frr:v8.4.0
  sudo docker pull timescale/timescaledb-ha:pg14-latest

  open --raw (
    nix build "git+ssh://git@github.com/altibiz/ozds?submodules=1#docker"
      --print-out-paths
      --no-link
  ) | sudo docker load
  open --raw (
    nix build "git+ssh://git@github.com/altibiz/pidgeon?submodules=1#default-docker"
      --print-out-paths
      --no-link
  ) | sudo docker load
  open --raw (
    nix build "git+ssh://git@github.com/altibiz/pidgeon?submodules=1#probe-docker"
      --print-out-paths
      --no-link
  ) | sudo docker load
}

def "main import" [--base: string = "http://localhost:3080" --meters: int = 10] {
  if $meters > 100 {
    print "Using more than 100 meters is not supported."
    exit 1
  }

  let meter_data = generate 0 { |i| 
    let type = $meter_types | get ($i mod ($meter_types | length))
    let serial = ($i + ($meter_types | length)) // ($meter_types | length)
    let node_id = random uuid
    let meter_id = $"($type)-($serial | to text)"
    let name = $"meter-($meter_id)"
    let port = 5100 + $i
    let link_id = random uuid
    let ip_address = $"10.0.0.(($i + 101) | to text)"
    let switch_id = $meter_switch_ids | get ($i mod ($meter_switch_ids | length))
    let switch_port = $i // ($meter_switch_ids | length)
    let image = "altibiz/pidgeon-probe:latest"
    {
      out: {
        node: ([
          { path: "console" value: $port }
          { path: "label.text" value: $name }
          { path: "name" value: $name }
          { path: "node_id" value: $node_id }
          { path: "properties.aux" value: $port }
          { path: "properties.container_id" value: (gen_container_id) }
          { path: "properties.image" value: $image }
          {
            path: "properties.start_command"
            value: $"pidgeon-probe-docker server -p 5020 -d ($type) -i ($serial) -m /assets"
          }
          { path: "x" value: (500 + ($i // 5) * 200) }
          { path: "y" value: (-300 + ($i mod 5) * 100) }
        ] | update path { |x|
          $x.path |
          split row "." |
          each { |x| try { $x | into int } catch { $x } } |
          into cell-path
        })
        link: ([
          { path: "link_id" value: $link_id }
          { path: "nodes.0.label.text" value: $"e($switch_port)" }
          { path: "nodes.0.node_id" value: $switch_id }
          { path: "nodes.0.port_number" value: $switch_port }
          { path: "nodes.1.label.text" value: "eth0" }
          { path: "nodes.1.node_id" value: $node_id }
          { path: "nodes.1.port_number" value: 0 }
        ] | update path { |x|
          $x.path |
          split row "." |
          each { |x| try { $x | into int } catch { $x } } |
          into cell-path
        })
        interfaces: {
          value: ($interfaces_template |
            str replace "{{generated}}" $ip_address)
          path: ([ $docker $node_id "etc" "network" "interfaces" ] |
            path join)
        }
      }
      next: ($i + 1)
    }
  } | first $meters

  cp -fr $test_dir $test_working_dir

  open $test_file |
    from json |
    update topology.nodes { |project|
      let meters = $meter_data | each { |properties|
        mut meter = $docker_node_template
        for property in $properties.node {
          $meter = ($meter | update $property.path $property.value)
        }
        mkdir ($properties.interfaces.path | path dirname)
        $properties.interfaces.value | save --force $properties.interfaces.path
        $meter
      }
      [ $project.topology.nodes $meters ] | flatten 
    } |
    update topology.links { |project|
      let links = $meter_data | each { |properties|
        mut link = $link_template
        for property in $properties.link {
          $link = ($link | update $property.path $property.value)
        }
        $link
      }
      [ $project.topology.links $links ] | flatten 
    } |
    update topology.nodes { |project|
      $project |
        get topology.nodes |
        each { |$node|
          if (($node | get --ignore-errors properties.extra_volumes) != null) {
            return ($node | update properties.extra_volumes { |node|
              $node.properties.extra_volumes | append "/assets"
            })
          }
          $node
        }
    } |
    to json |
    save -f $test_file
   
  ls $docker |
    each { |container|
      let container_assets = [ $container.name "assets" ] | path join
      cp -fr $assets $container_assets
    } | ignore

  cd $test_working_dir
  ^zip -r $test_zip . | ignore
  cd $working_dir

  curl -s -X DELETE $"($base)/v2/projects/($project_id)" | ignore
  (curl -s -X POST
    -H 'Content-Type:application/x-www-form-urlencoded'
    --data-binary $"@($test_zip)"
    $"($base)/v2/projects/($project_id)/import") | ignore

  print $"Imported project at: '($base)/static/web-ui/server/1/project/($project_id)'"
}

def "main export" [--base: string = "http://localhost:3080"] {
  curl -s -X POST $"($base)/v2/projects/($project_id)/close"
  curl -s $"($base)/v2/projects/($project_id)/export" -o $test_zip

  unzip $test_zip -d $test_working_dir | ignore

  open $test_file |
    from json |
    update topology { |project|
      let meter_node_ids = $project.topology.nodes
        | filter { |node|
          (($node.name | str starts-with "meter") and not
          ($node.name | str contains "switch"))
        }
        | get node_id
      $project.topology
        | update nodes { |topology| $topology.nodes
          | filter { |node| not ($meter_node_ids
            | any { |id| $id == $node.node_id })
          }
        }
        | update links { |topology| $topology.links
          | filter { |link| not ($link.nodes
            | any { |node| $meter_node_ids
              | any { |id| $id == $node.node_id }
            })
          }
        }
    } |
    update topology.nodes { |project|
      $project |
        get topology.nodes |
        each { |$node| 
          if ($node |
              get --ignore-errors properties.extra_volumes) != null {
            $node | update properties.extra_volumes { |node|
              $node.properties.extra_volumes | where { |x| $x != "/assets" }
            }
          } else {
            $node
          }
        }
    } |
    update topology.nodes { |project|
      $project.topology.nodes |
        each { |node|
          if ($node.node_type == "cloud") {
            $node | update properties.interfaces [ {
              name: "br-gns3"
              special: false
              type: "ethernet"
            } ]
          } else {
            $node
          }
        }
    } |
    to json |
    save -f $test_file

  let ids = open $test_file |
    from json |
    get topology.nodes.node_id
  ls $vpcs |
    filter { |x| not ($ids | any { |id| $id == ($x.name | path basename) }) } |
    each { |x| rm -fr $x.name } |
    ignore
  ls $docker | 
    filter { |x| not ($ids | any { |id| $id == ($x.name | path basename) }) } |
    each { |x| rm -fr $x.name } |
    ignore
  ls $docker |
    each { |x| rm -fr ([ $x.name "assets" ] | path join)} |
    ignore

  rm -fr $test_dir
  cp -fr $test_working_dir $test_dir
  prettier --write $orig_test_file | ignore

  print $"Exported project at: 'http://localhost:3080/static/web-ui/server/1/project/($project_id)'"
}

def "main dump" [host: string = "10.10.10.112"] {
  $env.PGHOST = $host;
  $env.PGPORT = "5432";
  $env.PGDATABASE = "ozds";
  $env.PGUSER = "ozds";
  $env.PGPASSWORD = "ozds";

  mut dump = (pg_dump
    --schema=public
    --exclude-table-data='*aggregates'
    --exclude-table-data='*measurements'
    --exclude-table-data='*invoices'
    --exclude-table-data='*calculations') |
    lines |
    where { |x| not ($x | str contains "timescaledb") }

  let hypertables = $dump |
    find --regex 'CREATE TABLE public\.\w+_(aggregates|measurements)' |
    (str replace --regex 
      'CREATE TABLE public\.(\w+_(?:aggregates|measurements)).*' 
      ("SELECT create_hypertable('\"$1\"', 'timestamp');\n"
        + "SELECT add_dimension('\"$1\"', 'meter_id', number_partitions => 2);"))

  [ $dump "SET search_path TO public, timescaledb;" $hypertables ] |
    flatten | str join "\n" | save -f $ozds
}
 
def "main stop" [base: string = "http://localhost:3080"] {
  curl -s -X POST $"($base)/v2/projects/($project_id)/close"
}

rm -f $test_working_dir
rm -f $test_zip
